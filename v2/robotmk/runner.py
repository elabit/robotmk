"""RetryStrategy. """

import dataclasses
import enum
import pathlib
import uuid
from collections.abc import Sequence
from typing import Final

PYTHON_EXECUTABLE: Final = pathlib.Path("python")


class RetryStrategy(enum.Enum):
    INCREMENTAL = "incremental"
    COMPLETE = "complete"


@dataclasses.dataclass(frozen=True)
class _RunnerSpec:
    robot_target: pathlib.Path
    outputdir: pathlib.Path
    output_name: str
    previous_output: pathlib.Path | None
    variablefile: pathlib.Path | None
    argumentfile: pathlib.Path | None
    retry_strategy: RetryStrategy

    def output(self) -> pathlib.Path:
        return self.outputdir / f"{self.output_name}.xml"

    def command(self) -> list[str]:
        robot_command = [str(PYTHON_EXECUTABLE), "-m", "robot"]
        if self.variablefile is not None:
            robot_command.append(f"--variablefile={self.variablefile}")
        if self.argumentfile is not None:
            robot_command.append(f"--argumentfile={self.argumentfile}")
        if self.retry_strategy is RetryStrategy.INCREMENTAL and self.previous_output:
            robot_command.append(f"--rerunfailed={self.previous_output}")
        return robot_command + [
            f"--outputdir={self.outputdir}",
            f"--output={self.output()}",
            str(self.robot_target),
        ]

    def _check(self) -> bool:
        paths_to_check = [
            self.robot_target,
            self.outputdir,
            self.variablefile,
            self.argumentfile,
            self.previous_output,
        ]
        return any(not path.exists() for path in paths_to_check if path is not None)


@dataclasses.dataclass(frozen=True)
class Variant:
    variablefile: pathlib.Path | None
    argumentfile: pathlib.Path | None


@dataclasses.dataclass(frozen=True)
class RetrySpec:
    id_: uuid.UUID
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    variants: Sequence[Variant]
    strategy: RetryStrategy

    def outputdir(self) -> pathlib.Path:
        return self.working_directory.joinpath(self.id_.hex)


@dataclasses.dataclass(frozen=True)
class Attempt:
    output: pathlib.Path
    command: list[str]


def create_attempts(spec: RetrySpec) -> list[Attempt]:
    attempts = []
    previous_output = None

    for i, variant in enumerate(spec.variants):
        runner_spec = _RunnerSpec(
            robot_target=spec.robot_target,
            outputdir=spec.outputdir(),
            output_name=str(
                i
            ),  # Ensure the `robot` command does not overwrite previous runs
            previous_output=previous_output,
            variablefile=variant.variablefile,
            argumentfile=variant.argumentfile,
            retry_strategy=spec.strategy,
        )
        previous_output = runner_spec.output()
        attempts.append(
            Attempt(
                output=runner_spec.output(),
                command=runner_spec.command(),
            )
        )

    return attempts
