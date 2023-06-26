import re
import os
from click.testing import CliRunner
import robotmk.cli.cli as cli

cwd = os.path.dirname(__file__)
robotmk_yml = os.path.join(cwd, "robotmk.yml")
robotmk_env = os.path.join(cwd, "robotmk.env")


def test_local_scheduler():
    """"""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["local", "--yml", robotmk_yml, "scheduler"])
    assert result.exit_code == 0


# test help message
def test_local_cli_help():
    """The help message should contain the three contexts and the help message itself."""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["local", "--help"])
    assert result.exit_code == 0
    # assert "Robotmk CLI Interface." in result.output
    assert re.search(r"--yml TEXT", result.output, re.DOTALL)
    assert not re.search(r"--vars TEXT", result.output, re.DOTALL)
    assert re.search(r"Commands:.*output.*scheduler", result.output, re.DOTALL)


def test_local_cli_invalid_vars_option():
    """The --vars option should be invalid for the local context."""
    runner = CliRunner()
    result = runner.invoke(
        cli.main, ["local", "--vars", robotmk_env, "--yml", robotmk_yml, "scheduler"]
    )
    assert result.exit_code == 2
    assert "Error: No such option: --vars" in result.output
