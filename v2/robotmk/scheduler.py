"""Scheduler"""

import argparse
import dataclasses
import datetime
import pathlib
import subprocess
from collections.abc import Iterable, Iterator, Mapping, Sequence
from typing import Final, Literal

from apscheduler.schedulers.blocking import BlockingScheduler  # type: ignore[import]
from apscheduler.triggers.interval import IntervalTrigger  # type: ignore[import]
from pydantic import BaseModel, TypeAdapter
from robot import rebot  # type: ignore[import]

from robotmk.api import Result
from robotmk.attempt import (
    Attempt,
    Identifier,
    RetrySpec,
    RetryStrategy,
    Variant,
    create_attempts,
)
from robotmk.environment import RCCEnvironment, ResultCode, RobotEnvironment
from robotmk.session import CurrentSession, UserSession


class _RCCConfig(BaseModel, frozen=True):
    rcc_binary_path: pathlib.Path


class _UserSessionConfig(BaseModel, frozen=True):
    user_name: str


class _SuiteConfig(BaseModel, frozen=True):
    execution_interval_seconds: int
    robot_target: pathlib.Path
    variants: Sequence[Variant]
    retry_strategy: RetryStrategy
    session: _UserSessionConfig | None


class _SystemPythonSuiteConfig(_SuiteConfig, frozen=True):
    ...


class _RCCSuiteConfig(_SuiteConfig, frozen=True):
    robot_yaml_path: pathlib.Path


@dataclasses.dataclass(frozen=True)
class _RCCEnvironmentSpec:
    binary_path: pathlib.Path
    robot_yaml_path: pathlib.Path


@dataclasses.dataclass(frozen=True)
class _SuiteSpecification:
    name: str  # ambiguous, since Robot Framework also provides names.
    config: _SuiteConfig
    rcc_env: _RCCEnvironmentSpec | None
    working_directory: pathlib.Path
    results_directory: pathlib.Path


class _ConfigSystemPython(BaseModel, frozen=True):
    environment: Literal["system_python"]
    working_directory: pathlib.Path
    results_directory: pathlib.Path
    suites: Mapping[str, _SystemPythonSuiteConfig]

    def suite_specifications(self) -> Iterator[_SuiteSpecification]:
        yield from (
            _SuiteSpecification(
                name=suite_name,
                config=suite_config,
                rcc_env=None,
                working_directory=self.working_directory,
                results_directory=self.results_directory,
            )
            for suite_name, suite_config in self.suites.items()
        )


class _ConfigRCC(BaseModel, frozen=True):
    environment: _RCCConfig
    working_directory: pathlib.Path
    results_directory: pathlib.Path
    suites: Mapping[str, _RCCSuiteConfig]

    def suite_specifications(self) -> Iterator[_SuiteSpecification]:
        yield from (
            _SuiteSpecification(
                name=suite_name,
                config=suite_config,
                rcc_env=_RCCEnvironmentSpec(
                    binary_path=self.environment.rcc_binary_path,
                    robot_yaml_path=suite_config.robot_yaml_path,
                ),
                working_directory=self.working_directory,
                results_directory=self.results_directory,
            )
            for suite_name, suite_config in self.suites.items()
        )


def _scheduler(config: _ConfigSystemPython | _ConfigRCC) -> BlockingScheduler:
    scheduler = BlockingScheduler()
    for suite_specification in config.suite_specifications():
        scheduler.add_job(
            _SuiteRetryRunner(suite_specification),
            name=suite_specification.name,
            trigger=IntervalTrigger(
                seconds=suite_specification.config.execution_interval_seconds
            ),
            next_run_time=datetime.datetime.now(),
        )
    return scheduler


def _environment(
    suite_name: str,
    config: _RCCEnvironmentSpec | None,
) -> RCCEnvironment | RobotEnvironment:
    if config is None:
        return RobotEnvironment()
    return RCCEnvironment(
        robot_yaml=config.robot_yaml_path,
        binary=config.binary_path,
        controller="robotmk",
        space=suite_name,
    )


def _session(
    suite_name: str,
    environment: _RCCEnvironmentSpec | None,
    session: _UserSessionConfig | None,
) -> CurrentSession | UserSession:
    env = _environment(suite_name, environment)
    if session:
        return UserSession(
            user_name=session.user_name,
            environment=env,
        )
    return CurrentSession(environment=env)


class _SuiteRetryRunner:  # pylint: disable=too-few-public-methods
    def __init__(self, suite_specification: _SuiteSpecification) -> None:
        self._suite_spec: Final = suite_specification
        self._session: Final = _session(
            suite_specification.name,
            suite_specification.rcc_env,
            suite_specification.config.session,
        )

    def __call__(self) -> None:
        retry_spec = RetrySpec(
            identifier=Identifier(
                name=self._suite_spec.name,
                timestamp=datetime.datetime.now(tz=datetime.timezone.utc).isoformat()
                # be compatible with Windows and Linux folder name restrictions
                .replace(":", "."),
            ),
            robot_target=self._suite_spec.config.robot_target,
            working_directory=self._suite_spec.working_directory,
            variants=self._suite_spec.config.variants,
            strategy=self._suite_spec.config.retry_strategy,
        )
        self._prepare_run(retry_spec.output_directory())

        outputs = self._run_attempts_until_successful(create_attempts(retry_spec))

        if not outputs:
            return  # Untested

        final_output = retry_spec.output_directory() / "merged.xml"
        rebot(*outputs, output=final_output, report=None, log=None)

        xml = final_output.read_text(encoding="utf-8")
        result = Result.create(retry_spec.identifier.name, xml)
        self._write_result_file_atomic(
            result=result,
            suite_working_directory=retry_spec.output_directory(),
        )

    def _prepare_run(self, output_dir: pathlib.Path) -> None:
        output_dir.mkdir(parents=True, exist_ok=True)
        if (build_command := self._session.environment.build_command()) is not None:
            _process = subprocess.run(build_command, check=True)

    def _run_attempts_until_successful(
        self,
        attempts: Iterable[Attempt],
    ) -> list[pathlib.Path]:
        outputs = []
        for attempt in attempts:
            match self._session.run(attempt):
                case ResultCode.ALL_TESTS_PASSED:
                    outputs.append(attempt.output_xml_file())
                case ResultCode.ROBOT_COMMAND_FAILED if attempt.output_xml_file().exists():
                    outputs.append(attempt.output_xml_file())
                    continue
            break
        return outputs

    def _write_result_file_atomic(
        self,
        *,
        result: Result,
        suite_working_directory: pathlib.Path,
    ) -> None:
        intermediate_result_path = suite_working_directory / "result.json"
        intermediate_result_path.write_text(
            result.model_dump_json(),
            encoding="utf-8",
        )
        intermediate_result_path.replace(
            _suite_result_file(
                _suite_results_directory(self._suite_spec.results_directory),
                self._suite_spec.name,
            )
        )


def _suite_results_directory(results_directory: pathlib.Path) -> pathlib.Path:
    return results_directory / "suites"


def _suite_result_file(
    suite_results_directory: pathlib.Path,
    suite_name: str,
) -> pathlib.Path:
    return suite_results_directory / f"{suite_name}.json"


def _setup(config: _ConfigSystemPython | _ConfigRCC) -> None:
    config.working_directory.mkdir(
        parents=True,
        exist_ok=True,
    )
    (suite_results_dir := _suite_results_directory(config.results_directory)).mkdir(
        parents=True,
        exist_ok=True,
    )
    _clean_up_results_directory_atomic(
        suite_results_directory=suite_results_dir,
        configured_suites=config.suites,
        intermediate_path_for_move=config.working_directory / "deprecated_result",
    )


def _clean_up_results_directory_atomic(
    *,
    suite_results_directory: pathlib.Path,
    configured_suites: Iterable[str],
    intermediate_path_for_move: pathlib.Path,
) -> None:
    for unwanted_result_file in set(suite_results_directory.iterdir()) - {
        _suite_result_file(suite_results_directory, suite_name)
        for suite_name in configured_suites
    }:
        unwanted_result_file.replace(intermediate_path_for_move)
    intermediate_path_for_move.unlink(missing_ok=True)


class Arguments(BaseModel, frozen=True):
    config_path: pathlib.Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("config_path", type=pathlib.Path)
    arguments = Arguments.model_validate(vars(parser.parse_args()))

    with arguments.config_path.open() as file:
        content = file.read()
    config = TypeAdapter(_ConfigSystemPython | _ConfigRCC).validate_json(content)
    # mypy somehow doesn't understand TypeAdapter.validate_json
    assert isinstance(config, _ConfigSystemPython | _ConfigRCC)

    _setup(config)
    _scheduler(config).start()


if __name__ == "__main__":
    main()
