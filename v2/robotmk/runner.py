"""RetryStrategy. """

import dataclasses
import enum
import pathlib
from collections.abc import Iterator, Sequence
from typing import Final

PYTHON_EXECUTABLE: Final = pathlib.Path("python")


@dataclasses.dataclass(frozen=True)
class Identifier:
    name: str
    timestamp: str


class RetryStrategy(enum.Enum):
    INCREMENTAL = "incremental"
    COMPLETE = "complete"


@dataclasses.dataclass(frozen=True)
class Variant:
    variablefile: pathlib.Path | None
    argumentfile: pathlib.Path | None


@dataclasses.dataclass(frozen=True)
class RetrySpec:
    identifier: Identifier
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    variants: Sequence[Variant]
    strategy: RetryStrategy

    def output_directory(self) -> pathlib.Path:
        return self.working_directory / self.identifier.name / self.identifier.timestamp


@dataclasses.dataclass(frozen=True)
class Attempt:
    output_directory: pathlib.Path
    identifier: Identifier
    index: int
    robot_target: pathlib.Path
    variable_file: pathlib.Path | None
    argument_file: pathlib.Path | None
    retry_strategy: RetryStrategy

    def output_file(self) -> pathlib.Path:
        return self._output_file(self.index)

    def command(self) -> list[str]:
        robot_command = [str(PYTHON_EXECUTABLE), "-m", "robot"]
        if self.variable_file is not None:
            robot_command.append(f"--variablefile={self.variable_file}")
        if self.argument_file is not None:
            robot_command.append(f"--argumentfile={self.argument_file}")
        if self.retry_strategy is RetryStrategy.INCREMENTAL and self.index > 0:
            robot_command.append(f"--rerunfailed={self._output_file(self.index - 1)}")
        return robot_command + [
            f"--outputdir={self.output_directory}",
            f"--output={self.output_file()}",
            str(self.robot_target),
        ]

    def _output_file(self, index: int) -> pathlib.Path:
        return self.output_directory.joinpath(f"{index}.xml")


def create_attempts(spec: RetrySpec) -> Iterator[Attempt]:
    yield from (
        Attempt(
            output_directory=spec.output_directory(),
            identifier=spec.identifier,
            index=i,
            robot_target=spec.robot_target,
            variable_file=variant.variablefile,
            argument_file=variant.argumentfile,
            retry_strategy=spec.strategy,
        )
        for i, variant in enumerate(spec.variants)
    )
