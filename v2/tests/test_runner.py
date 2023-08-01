import pathlib
import uuid

from robotmk import runner

# pylint: disable=protected-access


def test_create_command_complete() -> None:
    spec = runner._RunnerSpec(
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="0",
        previous_output=None,
        variablefile=None,
        argumentfile=None,
        retry_strategy=runner.RetryStrategy.COMPLETE,
    )
    expected = (
        "python -m robot --outputdir=/tmp/outputdir --output=/tmp/outputdir/0.xml "
        "~/suite/calculator.robot"
    )
    assert spec.command() == expected


def test_create_command_incremental() -> None:
    spec = runner._RunnerSpec(
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="1",
        previous_output=pathlib.Path("/tmp/outputdir/0.xml"),
        variablefile=None,
        argumentfile=pathlib.Path("~/suite/retry_arguments"),
        retry_strategy=runner.RetryStrategy.INCREMENTAL,
    )
    expected = (
        "python -m robot --argumentfile=~/suite/retry_arguments "
        "--rerunfailed=/tmp/outputdir/0.xml --outputdir=/tmp/outputdir "
        "--output=/tmp/outputdir/1.xml ~/suite/calculator.robot"
    )
    assert spec.command() == expected


def test_create_attempts() -> None:
    attempts = runner.create_attempts(
        runner.RetrySpec(
            id_=uuid.UUID("383783f4-1d02-43b1-9d6f-205f4d492d95"),
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            working_directory=pathlib.Path("/tmp/outputdir/"),
            schedule=[
                runner.Variant(
                    variablefile=None,
                    argumentfile=None,
                ),
                runner.Variant(
                    variablefile=pathlib.Path("~/suite/retry.yaml"),
                    argumentfile=None,
                ),
            ],
            strategy=runner.RetryStrategy.INCREMENTAL,
        )
    )
    assert attempts == [
        runner.Attempt(
            output=pathlib.Path(
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml"
            ),
            command="python -m robot "
            "--outputdir=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95 "
            "--output=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml "
            "~/suite/calculator.robot",
        ),
        runner.Attempt(
            output=pathlib.Path(
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/1.xml"
            ),
            command="python -m robot --variablefile=~/suite/retry.yaml "
            "--rerunfailed=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml "
            "--outputdir=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95 "
            "--output=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/1.xml "
            "~/suite/calculator.robot",
        ),
    ]


def test_create_merge_command() -> None:
    assert runner.create_merge_command(
        attempt_outputs=[
            pathlib.Path("/tmp/outputdir/0.xml"),
            pathlib.Path("/tmp/outputdir/1.xml"),
        ],
        final_output=pathlib.Path("/tmp/outputdir/merged.xml"),
    ) == (
        "python -m robot.rebot --output=/tmp/outputdir/merged.xml --report=NONE "
        "--log=NONE /tmp/outputdir/0.xml /tmp/outputdir/1.xml"
    )
