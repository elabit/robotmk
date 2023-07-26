"""Scheduler"""

import datetime
import pathlib
import shlex
import subprocess
from collections.abc import Iterable, Mapping, Sequence
from typing import Final
from uuid import uuid4

from apscheduler.schedulers.blocking import (  # type: ignore[import]
    BlockingScheduler,
)
from apscheduler.triggers.interval import (  # type: ignore[import]
    IntervalTrigger,
)
from pydantic import BaseModel
from runner import (
    Attempt,
    create_attempts,
    create_merge_command,
    RetrySpec,
    RetryStrategy,
    Variant,
)


class _SuiteConfig(BaseModel, frozen=True):  # pylint: disable=too-few-public-methods
    execution_interval_seconds: int
    python_executable: pathlib.Path
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    variants: Sequence[Variant]
    retry_strategy: RetryStrategy


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


class _SuiteRetryRunner:  # pylint: disable=too-few-public-methods
    def __init__(self, suite_config: _SuiteConfig) -> None:
        self._config: Final = suite_config

    def __call__(self) -> None:
        self._prepare_run()

        retry_spec = RetrySpec(
            id_=uuid4(),
            python_executable=self._config.python_executable,
            robot_target=self._config.robot_target,
            working_directory=self._config.working_directory,
            schedule=self._config.variants,
            strategy=self._config.retry_strategy,
        )

        outputs = self._run_attempts_until_successful(create_attempts(retry_spec))

        if not outputs:
            return

        subprocess.run(
            shlex.split(
                create_merge_command(
                    python_executable=self._config.python_executable,
                    attempt_outputs=outputs,
                    final_output=retry_spec.outputdir() / "merged.xml",
                )
            ),
            check=False,
        )

    def _prepare_run(self) -> None:
        self._config.working_directory.mkdir(parents=True, exist_ok=True)

    @staticmethod
    def _run_attempts_until_successful(
        attempts: Iterable[Attempt],
    ) -> list[pathlib.Path]:
        outputs = []
        for attempt in attempts:
            completed_process = subprocess.run(
                shlex.split(attempt.command), check=False
            )
            if completed_process.returncode <= 250:
                outputs.append(attempt.output)
            if completed_process.returncode == 0:
                break
        return outputs
