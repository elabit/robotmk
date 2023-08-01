import dataclasses
import enum
import pathlib
import subprocess


class ResultCode(enum.Enum):
    ALL_TESTS_PASSED = "all_tests_passed"
    ROBOT_COMMAND_FAILED = "robot_command_failed"
    RCC_ERROR = "rcc_error"


@dataclasses.dataclass(frozen=True)
class RCCEnvironment:
    robot_yaml: pathlib.Path
    binary: str

    def build_command(self) -> str:
        return f"{self.binary} holotree variables --json -r {self.robot_yaml}"

    def extend(self, robot_command: str) -> str:
        return f"{self.binary} task script -r {self.robot_yaml} -- {robot_command}"

    @staticmethod
    def create_result_code(process: subprocess.CompletedProcess[str]) -> ResultCode:
        if process.returncode == 0:
            return ResultCode.ALL_TESTS_PASSED
        if process.returncode == 10:
            return ResultCode.ROBOT_COMMAND_FAILED
        return ResultCode.RCC_ERROR


@dataclasses.dataclass(frozen=True)
class RobotEnvironment:
    def build_command(self) -> None:
        return None

    def extend(self, robot_command: str) -> str:
        return robot_command

    @staticmethod
    def create_result_code(process: subprocess.CompletedProcess[str]) -> ResultCode:
        if process.returncode == 0:
            return ResultCode.ALL_TESTS_PASSED
        return ResultCode.ROBOT_COMMAND_FAILED
