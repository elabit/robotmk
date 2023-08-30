import pathlib

from robotmk import runner


def test_create_command_complete() -> None:
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=runner.Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=0,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=runner.RetryStrategy.COMPLETE,
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
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=runner.Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=0,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=runner.RetryStrategy.INCREMENTAL,
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
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/my_suite/2023-08-29T12.23.44.419347+00.00"),
        identifier=runner.Identifier(
            name="my_suite",
            timestamp="2023-08-29T12.23.44.419347+00.00",
        ),
        index=1,
        robot_target=pathlib.Path("~/suite/calculator.robot"),
        variable_file=None,
        argument_file=None,
        retry_strategy=runner.RetryStrategy.INCREMENTAL,
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
        runner.create_attempts(
            runner.RetrySpec(
                identifier=runner.Identifier(
                    name="suite_1",
                    timestamp="2023-08-29T12.23.44.419347+00.00",
                ),
                robot_target=pathlib.Path("~/suite/calculator.robot"),
                working_directory=pathlib.Path("/tmp/outputdir/"),
                variants=[
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
    )
    assert attempts == [
        runner.Attempt(
            output_directory=pathlib.Path(
                "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00"
            ),
            identifier=runner.Identifier(
                name="suite_1",
                timestamp="2023-08-29T12.23.44.419347+00.00",
            ),
            index=0,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=None,
            argument_file=None,
            retry_strategy=runner.RetryStrategy.INCREMENTAL,
        ),
        runner.Attempt(
            output_directory=pathlib.Path(
                "/tmp/outputdir/suite_1/2023-08-29T12.23.44.419347+00.00"
            ),
            identifier=runner.Identifier(
                name="suite_1",
                timestamp="2023-08-29T12.23.44.419347+00.00",
            ),
            index=1,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=pathlib.Path("~/suite/retry.yaml"),
            argument_file=None,
            retry_strategy=runner.RetryStrategy.INCREMENTAL,
        ),
    ]
