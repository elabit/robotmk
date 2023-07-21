import pathlib

import runner


def test_create_command() -> None:
    spec = runner._RunnerSpec(  # pylint: disable=protected-access
        python_executable=pathlib.Path("/usr/bin/python3"),
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        outputdir=pathlib.Path("/tmp/outputdir/"),
        output="output",
        variablefile=None,
        argumentfile=None,
    )
    command = runner._create_command(spec)  # pylint: disable=protected-access
    expected = (
        "/usr/bin/python3 -m robot --outputdir=/tmp/outputdir --output=output "
        "~/suite/calculator.robot"
    )
    assert command == expected
