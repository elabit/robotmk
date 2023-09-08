from __future__ import annotations

import subprocess
import time
from collections.abc import Iterable
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path

from robotmk.attempt import Attempt
from robotmk.environment import RCCEnvironment, ResultCode, SystemEnvironment


@dataclass(frozen=True)
class CurrentSession:
    environment: RCCEnvironment | SystemEnvironment

    def run(self, attempt: Attempt) -> ResultCode:
        return self.environment.create_result_code(
            subprocess.run(
                self.environment.wrap_for_execution(attempt.command()),
                check=False,
                encoding="utf-8",
            ).returncode
        )


@dataclass(frozen=True)
class UserSession:
    user_name: str
    environment: RCCEnvironment | SystemEnvironment

    def run(self, attempt: Attempt) -> ResultCode:
        task, exit_code_path = self._prepare_run(attempt)
        task_scheduler = _TaskScheduler(task)

        task_scheduler.create_task()
        task_scheduler.run_task()
        while self._task_is_running(task_scheduler):
            time.sleep(10)
        task_scheduler.delete_task()

        return self.environment.create_result_code(
            int(exit_code_path.read_text(encoding="utf-8").split()[0])
        )

    def _prepare_run(self, attempt: Attempt) -> tuple[_Task, Path]:
        # NOTE: The .bat-suffix is important! Without, schtasks doesn't know how to run this.
        script_path = attempt.output_directory / f"{attempt.index}_execute.bat"
        exit_code_path = attempt.output_directory / f"{attempt.index}_exit_code"
        script_path.write_text(
            self._build_task_script(
                cmd=self.environment.wrap_for_execution(attempt.command()),
                exit_code_path=exit_code_path,
            ),
            encoding="utf-8",
        )
        return (
            _Task(
                task_name="robotmk-"
                f"{attempt.identifier.name}-"
                f"{attempt.identifier.timestamp}-"
                f"{attempt.index}",
                script_path=script_path,
                user_name=self.user_name,
            ),
            exit_code_path,
        )

    @staticmethod
    def _build_task_script(*, cmd: Iterable[str], exit_code_path: Path) -> str:
        return "\n".join(
            [
                "@echo off",
                " ".join(cmd),
                f"echo %errorlevel% >{exit_code_path}",
            ]
        )

    @staticmethod
    def _task_is_running(task_scheduler: _TaskScheduler) -> bool:
        return "Running" in task_scheduler.query_task().stdout


@dataclass(frozen=True)
class _Task:
    task_name: str
    script_path: Path
    user_name: str


@dataclass(frozen=True)
class _TaskScheduler:
    task: _Task

    def create_task(self) -> subprocess.CompletedProcess[str]:
        return self._run_schtasks(
            [
                "/create",
                "/tn",
                self.task.task_name,
                "/tr",
                self.task.script_path,
                "/sc",
                "ONCE",
                "/ru",
                self.task.user_name,
                "/it",
                "/rl",
                "LIMITED",
                "/st",
                # Since we are forced to provide this option, ensure that the task does not
                # accidentally start because we hit the start time.
                (datetime.now() - timedelta(seconds=60)).strftime("%H:%M"),
                "/f",
            ],
        )

    def run_task(self) -> subprocess.CompletedProcess[str]:
        return self._run_schtasks(
            [
                "/run",
                "/tn",
                self.task.task_name,
            ],
        )

    def query_task(self) -> subprocess.CompletedProcess[str]:
        return self._run_schtasks(
            [
                "/query",
                "/tn",
                self.task.task_name,
                "/fo",
                "CSV",
                "/nh",
            ],
        )

    def delete_task(self) -> subprocess.CompletedProcess[str]:
        return self._run_schtasks(
            [
                "/delete",
                "/tn",
                self.task.task_name,
                "/f",
            ],
        )

    @staticmethod
    def _run_schtasks(
        arguments: Iterable[str | Path],
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                "schtasks",
                *arguments,
            ],
            check=True,
            capture_output=True,
            encoding="utf-8",
        )
