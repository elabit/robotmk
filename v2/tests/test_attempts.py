import pathlib

from robotmk.attempt import Attempt, Identifier, RetrySpec, create_attempts
from robotmk.config import RetryStrategy, Variant


def test_create_command_complete() -> None:
    attempt = Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=0,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=RetryStrategy.COMPLETE,
    )
    expected = [
        "python",
        "-m",
        "robot",
        "--outputdir=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00",
        "--output=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/0.xml",
        "--log=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/0.html",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_command_incremental_first() -> None:
    attempt = Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=0,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=RetryStrategy.INCREMENTAL,
    )
    expected = [
        "python",
        "-m",
        "robot",
        "--outputdir=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00",
        "--output=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/0.xml",
        "--log=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/0.html",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_command_incremental_second() -> None:
    attempt = Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=1,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=RetryStrategy.INCREMENTAL,
    )
    expected = [
        "python",
        "-m",
        "robot",
        "--rerunfailed=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/0.xml",
        "--outputdir=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00",
        "--output=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/1.xml",
        "--log=/tmp/my_suite/2023-08-29T12.23.44.419347+00.00/1.html",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_attempts() -> None:
    attempts = list(
        create_attempts(
            RetrySpec(
                identifier=Identifier(
                    name="suite_1",
                    timestamp="2023-08-29T12.23.44.419347+00.00",
                ),
                robot_target=pathlib.Path("~/suite/calculator.robot"),
                working_directory=pathlib.Path("/tmp/outputdir/"),
                variants=[
                    Variant(
                        variablefile=None,
                        argumentfile=None,
                    ),
                    Variant(
                        variablefile=pathlib.Path("~/suite/retry.yaml"),
                        argumentfile=None,
                    ),
                ],
                strategy=RetryStrategy.INCREMENTAL,
            )
        )
    )
    assert attempts == [
        Attempt(
            output_directory=pathlib.Path(
                "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00"
            ),
            identifier=Identifier(
                name="suite_1",
                timestamp="2023-08-29T12.23.44.419347+00.00",
            ),
            index=0,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=None,
            argument_file=None,
            retry_strategy=RetryStrategy.INCREMENTAL,
        ),
        Attempt(
            output_directory=pathlib.Path(
                "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00"
            ),
            identifier=Identifier(
                name="suite_1",
                timestamp="2023-08-29T12.23.44.419347+00.00",
            ),
            index=1,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=pathlib.Path("~/suite/retry.yaml"),
            argument_file=None,
            retry_strategy=RetryStrategy.INCREMENTAL,
        ),
    ]
