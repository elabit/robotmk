import pathlib

import runner


def test_create_command_complete() -> None:
    spec = runner._RunnerSpec(  # pylint: disable=protected-access
        python_executable=pathlib.Path("/usr/bin/python3"),
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="0",
        previous_output=None,
        variablefile=None,
        argumentfile=None,
        retry_strategy=runner._RetryStrategy.COMPLETE,  # pylint: disable=protected-access
    )
    command = runner._create_command(spec)  # pylint: disable=protected-access
    expected = (
        "/usr/bin/python3 -m robot --outputdir=/tmp/outputdir --output=/tmp/outputdir/0.xml "
        "~/suite/calculator.robot"
    )
    assert command == expected


def test_create_command_incremental() -> None:
    spec = runner._RunnerSpec(  # pylint: disable=protected-access
        python_executable=pathlib.Path("/usr/bin/python3"),
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output_name="1",
        previous_output=pathlib.Path("/tmp/outputdir/0.xml"),
        variablefile=None,
        argumentfile=pathlib.Path("~/suite/retry_arguments"),
        retry_strategy=runner._RetryStrategy.INCREMENTAL,  # pylint: disable=protected-access
    )
    command = runner._create_command(spec)  # pylint: disable=protected-access
    expected = (
        "/usr/bin/python3 -m robot --argumentfile=~/suite/retry_arguments "
        "--rerunfailed=/tmp/outputdir/0.xml --outputdir=/tmp/outputdir "
        "--output=/tmp/outputdir/1.xml ~/suite/calculator.robot"
    )
    assert command == expected
