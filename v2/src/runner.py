"""RetryStrategy. """

import dataclasses
import enum
import pathlib
import uuid
from collections.abc import Iterable, Sequence


class RetryStrategy(enum.Enum):
    INCREMENTAL = "incremental"
    COMPLETE = "complete"


@dataclasses.dataclass(frozen=True)
class _RunnerSpec:  # pylint: disable=too-many-instance-attributes
    python_executable: pathlib.Path
    robot_target: pathlib.Path
    outputdir: pathlib.Path
    output_name: str
    previous_output: pathlib.Path | None
    variablefile: pathlib.Path | None
    argumentfile: pathlib.Path | None
    retry_strategy: RetryStrategy

    def output(self) -> pathlib.Path:
        return self.outputdir / f"{self.output_name}.xml"

    def command(self) -> str:
        robot_command = f"{self.python_executable} -m robot "
        if self.variablefile is not None:
            robot_command += f"--variablefile={self.variablefile} "
        if self.argumentfile is not None:
            robot_command += f"--argumentfile={self.argumentfile} "
        if self.retry_strategy is RetryStrategy.INCREMENTAL and self.previous_output:
            robot_command += f"--rerunfailed={self.previous_output} "
        return robot_command + (
            f"--outputdir={self.outputdir} "
            f"--output={self.output()} "
            f"{self.robot_target}"
        )

    def _check(self) -> bool:
        paths_to_check = [
            self.python_executable,
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
    python_executable: pathlib.Path
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    schedule: Sequence[Variant]
    strategy: RetryStrategy

    def outputdir(self) -> pathlib.Path:
        return self.working_directory.joinpath(self.id_.hex)


@dataclasses.dataclass(frozen=True)
class Attempt:
    output: pathlib.Path
    command: str


def create_attempts(spec: RetrySpec) -> list[Attempt]:
    attempts = []
    previous_output = None

    for i, variant in enumerate(spec.schedule):
        runner_spec = _RunnerSpec(
            python_executable=spec.python_executable,
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


def create_merge_command(
    *,
    python_executable: pathlib.Path,
    attempt_outputs: Iterable[pathlib.Path],
    final_output: pathlib.Path,
) -> str:
    return (
        f"{python_executable} -m robot.rebot --output={final_output} --report=NONE --log=NONE "
        + " ".join(str(variant_output) for variant_output in attempt_outputs)
    )
