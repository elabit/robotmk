import pathlib
import uuid

from robotmk import runner


def test_create_command_complete() -> None:
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/d9e87a17-2e68-450a-8228-624604d47b26/"),
        id_=uuid.UUID("d9e87a17-2e68-450a-8228-624604d47b26"),
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
        "--outputdir=/tmp/d9e87a17-2e68-450a-8228-624604d47b26",
        "--output=/tmp/d9e87a17-2e68-450a-8228-624604d47b26/0.xml",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_command_incremental_first() -> None:
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/d9e87a17-2e68-450a-8228-624604d47b26/"),
        id_=uuid.UUID("d9e87a17-2e68-450a-8228-624604d47b26"),
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
        "--outputdir=/tmp/d9e87a17-2e68-450a-8228-624604d47b26",
        "--output=/tmp/d9e87a17-2e68-450a-8228-624604d47b26/0.xml",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_command_incremental_second() -> None:
    attempt = runner.Attempt(
        output_directory=pathlib.Path("/tmp/d9e87a17-2e68-450a-8228-624604d47b26/"),
        id_=uuid.UUID("d9e87a17-2e68-450a-8228-624604d47b26"),
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
        "--rerunfailed=/tmp/d9e87a17-2e68-450a-8228-624604d47b26/0.xml",
        "--outputdir=/tmp/d9e87a17-2e68-450a-8228-624604d47b26",
        "--output=/tmp/d9e87a17-2e68-450a-8228-624604d47b26/1.xml",
        "~/suite/calculator.robot",
    ]
    assert attempt.command() == expected


def test_create_attempts() -> None:
    attempts = list(
        runner.create_attempts(
            runner.RetrySpec(
                id_=uuid.UUID("383783f4-1d02-43b1-9d6f-205f4d492d95"),
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
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95"
            ),
            id_=uuid.UUID("383783f4-1d02-43b1-9d6f-205f4d492d95"),
            index=0,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=None,
            argument_file=None,
            retry_strategy=runner.RetryStrategy.INCREMENTAL,
        ),
        runner.Attempt(
            output_directory=pathlib.Path(
                "/tmp/outputdir/383783f41d0243b19d6f205f4d492d95"
            ),
            id_=uuid.UUID("383783f4-1d02-43b1-9d6f-205f4d492d95"),
            index=1,
            robot_target=pathlib.Path("~/suite/calculator.robot"),
            variable_file=pathlib.Path("~/suite/retry.yaml"),
            argument_file=None,
            retry_strategy=runner.RetryStrategy.INCREMENTAL,
        ),
    ]
