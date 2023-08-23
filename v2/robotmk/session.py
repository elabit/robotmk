import subprocess
from dataclasses import dataclass

from .environment import RCCEnvironment, ResultCode, RobotEnvironment
from .runner import Attempt


@dataclass(frozen=True)
class CurrentSession:
    environment: RCCEnvironment | RobotEnvironment

    def run(self, attempt: Attempt) -> ResultCode:
        return self.environment.create_result_code(
            subprocess.run(
                self.environment.wrap_for_execution(attempt.command()),
                check=False,
                encoding="utf-8",
            )
        )
