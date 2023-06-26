import re
import os
from click.testing import CliRunner
import robotmk.cli.cli as cli

cwd = os.path.dirname(__file__)
robotmk_yml = os.path.join(cwd, "robotmk.yml")
robotmk_env = os.path.join(cwd, "robotmk.env")


# test help message
def test_specialagent_cli_help():
    """The help message should contain the three contexts and the help message itself."""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["specialagent", "--help"])
    assert result.exit_code == 0
    # assert "Robotmk CLI Interface." in result.output
    assert not re.search(r"--yml TEXT", result.output, re.DOTALL)
    assert re.search(r"--vars TEXT", result.output, re.DOTALL)
    assert re.search(r"Commands:.*output.*sequencer", result.output, re.DOTALL)
