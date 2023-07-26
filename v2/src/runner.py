"""RetryStrategy. """

import dataclasses
import enum
import pathlib
import uuid
from collections.abc import Sequence


class _RetryStrategy(enum.Enum):
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
    retry_strategy: _RetryStrategy

    def output(self) -> pathlib.Path:
        return self.outputdir / f"{self.output_name}.xml"

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
class _Variant:
    variablefile: pathlib.Path | None
    argumentfile: pathlib.Path | None


@dataclasses.dataclass(frozen=True)
class _RetrySpec:
    id_: uuid.UUID
    python_executable: pathlib.Path
    robot_target: pathlib.Path
    working_directory: pathlib.Path
    schedule: Sequence[_Variant]
    strategy: _RetryStrategy

    def outputdir(self) -> pathlib.Path:
        return self.working_directory.joinpath(self.id_.hex)


def _create_command(spec: _RunnerSpec) -> str:
    robot_command = f"{spec.python_executable} -m robot "
    if spec.variablefile is not None:
        robot_command += f"--variablefile={spec.variablefile} "
    if spec.argumentfile is not None:
        robot_command += f"--argumentfile={spec.argumentfile} "
    if spec.retry_strategy is _RetryStrategy.INCREMENTAL and spec.previous_output:
        robot_command += f"--rerunfailed={spec.previous_output} "
    return robot_command + (
        f"--outputdir={spec.outputdir} "
        f"--output={spec.output()} "
        f"{spec.robot_target}"
    )


def _create_commands(spec: _RetrySpec) -> Sequence[str]:
    commands = []
    previous_output = None
    for i, variant in enumerate(spec.schedule):
        runner_cfg = _RunnerSpec(
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
        commands.append(_create_command(runner_cfg))
        previous_output = runner_cfg.output()
    return commands
