import pathlib
import uuid

import runner

# pylint: disable=protected-access


def test_create_command_complete() -> None:
    spec = runner._RunnerSpec(
        python_executable=pathlib.Path("/usr/bin/python3"),
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="0",
        previous_output=None,
        variablefile=None,
        argumentfile=None,
        retry_strategy=runner._RetryStrategy.COMPLETE,
    )
    expected = (
        "/usr/bin/python3 -m robot --outputdir=/tmp/outputdir --output=/tmp/outputdir/0.xml "
        "~/suite/calculator.robot"
    )
    assert spec.command() == expected


def test_create_command_incremental() -> None:
    spec = runner._RunnerSpec(
        python_executable=pathlib.Path("/usr/bin/python3"),
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="1",
        previous_output=pathlib.Path("/tmp/outputdir/0.xml"),
        variablefile=None,
        argumentfile=pathlib.Path("~/suite/retry_arguments"),
        retry_strategy=runner._RetryStrategy.INCREMENTAL,
    )
    expected = (
        "/usr/bin/python3 -m robot --argumentfile=~/suite/retry_arguments "
        "--rerunfailed=/tmp/outputdir/0.xml --outputdir=/tmp/outputdir "
        "--output=/tmp/outputdir/1.xml ~/suite/calculator.robot"
    )
    assert spec.command() == expected


def test_create_attempts() -> None:
    attempts = runner._create_attempts(
        runner._RetrySpec(
            id_=uuid.UUID("383783f4-1d02-43b1-9d6f-205f4d492d95"),
            python_executable=pathlib.Path("/usr/bin/python3"),
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            working_directory=pathlib.Path("/tmp/outputdir/"),
            schedule=[
                runner._Variant(
                    variablefile=None,
                    argumentfile=None,
                ),
                runner._Variant(
                    variablefile=pathlib.Path("~/suite/retry.yaml"),
                    argumentfile=None,
                ),
            ],
            strategy=runner._RetryStrategy.INCREMENTAL,
        )
    )
    assert attempts == [
        runner._Attempt(
            output=pathlib.Path(
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml"
            ),
            command="/usr/bin/python3 -m robot "
            "--outputdir=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95 "
            "--output=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml "
            "~/suite/calculator.robot",
        ),
        runner._Attempt(
            output=pathlib.Path(
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/1.xml"
            ),
            command="/usr/bin/python3 -m robot --variablefile=~/suite/retry.yaml "
            "--rerunfailed=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/0.xml "
            "--outputdir=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95 "
            "--output=/tmp/outputdir/383783f41d0243b19d6f205f4d492d95/1.xml "
            "~/suite/calculator.robot",
        ),
    ]


def test_create_merge_command() -> None:
    assert runner._create_merge_command(
        python_executable=pathlib.Path("/usr/bin/python3"),
        attempt_outputs=[
            pathlib.Path("/tmp/outputdir/0.xml"),
            pathlib.Path("/tmp/outputdir/1.xml"),
        ],
        final_output=pathlib.Path("/tmp/outputdir/merged.xml"),
    ) == (
        "/usr/bin/python3 -m robot.rebot --output=/tmp/outputdir/merged.xml --report=NONE "
        "--log=NONE /tmp/outputdir/0.xml /tmp/outputdir/1.xml"
    )
