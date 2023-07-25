import pathlib
import uuid

import rccer
import runner


def test_create_rcc_runners() -> None:
    # Assemble
    rcc_dir = pathlib.Path("~/git/robotmk/v2/data/minimal_rcc/")
    robot_yaml = rcc_dir.joinpath("robot_yaml")
    tasks_robot = rcc_dir.joinpath("task.robot")

    variant = runner._Variant(  # pylint: disable=protected-access
        variablefile=None,
        argumentfile=None,
    )
    retry_example = runner.RetrySpec(
        id_=uuid.UUID("4f3771f3-60db-48c0-aec6-48d40fbffd5c"),
        python_executable=pathlib.Path("python"),
        robot_target=tasks_robot,
        working_directory=pathlib.Path("/tmp/retry"),
        schedule=[variant, variant],
    )
    commands = runner.create_commands(retry_example)
    # Act
    rcc_runners = rccer.create_rcc_runners("rcc", robot_yaml, retry_example)
    # Assert
    assert [rcc_runner.create_command() for rcc_runner in rcc_runners] == [
        f"rcc task script -r {robot_yaml} -- {commands[0]}",
        f"rcc task script -r {robot_yaml} -- {commands[1]}",
    ]
