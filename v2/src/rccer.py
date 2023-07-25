import dataclasses
import pathlib
import subprocess

import runner


@dataclasses.dataclass(frozen=True)
class _RCCBuilder:
    rcc_binary: str
    robot_yaml: pathlib.Path

    def create_command(self) -> str:
        return f"{self.rcc_binary} holotree variables --json -r {self.robot_yaml}"


@dataclasses.dataclass(frozen=True)
class _RCCRunner:
    rcc_binary: str
    robot_yaml: pathlib.Path
    command: str

    def create_command(self) -> str:
        return f"{self.rcc_binary} task script -r {self.robot_yaml} -- {self.command}"


def _execute(target: _RCCRunner | _RCCBuilder) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        target.create_command(),
        shell=True,
        check=False,
        capture_output=True,
        encoding="utf-8",
    )


def _create_rcc_runners(
    rcc_binary: str, robot_yaml: pathlib.Path, example: runner.RetrySpec
) -> list[_RCCRunner]:
    commands = runner.create_commands(example)
    return [
        _RCCRunner(rcc_binary=rcc_binary, robot_yaml=robot_yaml, command=command)
        for command in commands
    ]
