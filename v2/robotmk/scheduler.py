"""Scheduler"""

import argparse
import datetime
import pathlib
import subprocess
from collections.abc import Iterable, Mapping, Sequence
from typing import Final
from uuid import uuid4

from apscheduler.schedulers.blocking import BlockingScheduler  # type: ignore[import]
from apscheduler.triggers.interval import IntervalTrigger  # type: ignore[import]
from pydantic import BaseModel
from robot import rebot  # type: ignore[import]

from robotmk.environment import RCCEnvironment, ResultCode, RobotEnvironment
from robotmk.runner import Attempt, RetrySpec, RetryStrategy, Variant, create_attempts


class _RCC(BaseModel, frozen=True):
    binary: pathlib.Path
    robot_yaml: pathlib.Path


class _SuiteConfig(BaseModel, frozen=True):  # pylint: disable=too-few-public-methods
    execution_interval_seconds: int
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    variants: Sequence[Variant]
    retry_strategy: RetryStrategy
    env: _RCC | None


class _SchedulerConfig(BaseModel, frozen=True):
    suites: Mapping[str, _SuiteConfig]


def _scheduler(suites: Mapping[str, _SuiteConfig]) -> BlockingScheduler:
    scheduler = BlockingScheduler()
    for suite_name, suite_config in suites.items():
        scheduler.add_job(
            _SuiteRetryRunner(suite_config),
            name=suite_name,
            trigger=IntervalTrigger(seconds=suite_config.execution_interval_seconds),
            next_run_time=datetime.datetime.now(),
        )
    return scheduler


def _environment(config: _RCC | None) -> RCCEnvironment | RobotEnvironment:
    if config is None:
        return RobotEnvironment()
    return RCCEnvironment(robot_yaml=config.robot_yaml, binary=config.binary)


class _SuiteRetryRunner:  # pylint: disable=too-few-public-methods
    def __init__(self, suite_config: _SuiteConfig) -> None:
        self._config: Final = suite_config
        self._env: Final = _environment(suite_config.env)
        self._final_outputs: list[pathlib.Path] = []

    def __call__(self) -> None:
        self._prepare_run()

        retry_spec = RetrySpec(
            id_=uuid4(),
            robot_target=self._config.robot_target,
            working_directory=self._config.working_directory,
            variants=self._config.variants,
            strategy=self._config.retry_strategy,
        )

        outputs = self._run_attempts_until_successful(create_attempts(retry_spec))

        if not outputs:
            return  # Untested

        final_output = retry_spec.outputdir() / "merged.xml"

        rebot(*outputs, output=final_output, report=None, log=None)
        self._final_outputs.append(final_output)

    def _prepare_run(self) -> None:
        self._config.working_directory.mkdir(parents=True, exist_ok=True)
        if (build_command := self._env.build_command()) is not None:
            _process = subprocess.run(build_command, check=True)

    def _run_attempts_until_successful(
        self,
        attempts: Iterable[Attempt],
    ) -> list[pathlib.Path]:
        outputs = []
        for attempt in attempts:
            command = self._env.wrap_for_execution(attempt.command)
            process = subprocess.run(command, check=False, encoding="utf-8")
            match self._env.create_result_code(process):
                case ResultCode.ALL_TESTS_PASSED:
                    outputs.append(attempt.output)
                case ResultCode.ROBOT_COMMAND_FAILED if attempt.output.exists():
                    outputs.append(attempt.output)
                    continue
            break
        return outputs


class Arguments(BaseModel, frozen=True):
    config_path: pathlib.Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("config_path", type=pathlib.Path)
    arguments = Arguments.model_validate(vars(parser.parse_args()))

    with arguments.config_path.open() as file:
        content = file.read()
    config = _SchedulerConfig.model_validate_json(content)
    _scheduler(config.suites).start()


if __name__ == "__main__":
    main()
