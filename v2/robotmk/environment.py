import dataclasses
import enum
import pathlib
from collections.abc import Sequence


class ResultCode(enum.Enum):
    ALL_TESTS_PASSED = "all_tests_passed"
    ROBOT_COMMAND_FAILED = "robot_command_failed"
    RCC_ERROR = "rcc_error"


@dataclasses.dataclass(frozen=True)
class RCCEnvironment:
    robot_yaml: pathlib.Path
    binary: pathlib.Path
    controller: str
    space: str

    def build_command(self) -> list[str]:
        return [
            str(self.binary),
            "holotree",
            "variables",
            *self._controller_and_space_args(),
            "--json",
            "-r",
            str(self.robot_yaml),
        ]

    def wrap_for_execution(self, command: Sequence[str]) -> list[str]:
        rcc_command = [
            str(self.binary),
            "task",
            "script",
            *self._controller_and_space_args(),
            "-r",
            str(self.robot_yaml),
            "--",
        ]
        return [
            *rcc_command,
            *command,
        ]

    @staticmethod
    def create_result_code(exit_code: int) -> ResultCode:
        if exit_code == 0:
            return ResultCode.ALL_TESTS_PASSED
        if exit_code == 10:
            return ResultCode.ROBOT_COMMAND_FAILED
        return ResultCode.RCC_ERROR

    def _controller_and_space_args(self) -> list[str]:
        return [
            "--controller",
            self.controller,
            "--space",
            self.space,
        ]


@dataclasses.dataclass(frozen=True)
class RobotEnvironment:
    def build_command(self) -> None:
        return None

    def wrap_for_execution(self, command: Sequence[str]) -> Sequence[str]:
        return command

    @staticmethod
    def create_result_code(exit_code: int) -> ResultCode:
        if exit_code == 0:
            return ResultCode.ALL_TESTS_PASSED
        return ResultCode.ROBOT_COMMAND_FAILED
