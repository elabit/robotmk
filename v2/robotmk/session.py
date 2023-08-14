import subprocess
import time
from collections.abc import Iterable
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path

from robotmk.environment import RCCEnvironment, ResultCode, RobotEnvironment
from robotmk.runner import Attempt


@dataclass(frozen=True)
class CurrentSession:
    environment: RCCEnvironment | RobotEnvironment

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
    environment: RCCEnvironment | RobotEnvironment

    def run(self, attempt: Attempt) -> ResultCode:
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
        task_name = f"robotmk-{attempt.id_}-{attempt.index}"

        subprocess.run(
            [
                "schtasks",
                "/create",
                "/tn",
                task_name,
                "/tr",
                script_path,
                "/sc",
                "ONCE",
                "/ru",
                self.user_name,
                "/it",
                "/rl",
                "LIMITED",
                "/st",
                (datetime.now() - timedelta(seconds=60)).strftime("%H:%M"),
                "/f",
            ],
            check=True,
        )

        subprocess.run(
            [
                "schtasks",
                "/run",
                "/tn",
                task_name,
            ],
            check=True,
        )

        while self._task_is_running(task_name):
            time.sleep(10)

        subprocess.run(
            [
                "schtasks",
                "/delete",
                "/tn",
                task_name,
                "/f",
            ],
            check=True,
        )

        return self.environment.create_result_code(
            int(exit_code_path.read_text(encoding="utf-8").split()[0])
        )

    @staticmethod
    def _task_is_running(task_name: str) -> bool:
        return (
            "Running"
            in subprocess.run(
                [
                    "schtasks",
                    "/query",
                    "/tn",
                    task_name,
                    "/fo",
                    "CSV",
                    "/nh",
                ],
                check=True,
                capture_output=True,
                encoding="utf-8",
            ).stdout
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
