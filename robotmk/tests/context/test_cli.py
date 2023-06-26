from click.testing import CliRunner
import robotmk.cli.cli as cli
import re


# test help message
def test_cli_help():
    """The help message shoudl contain the three contexts and the help message itself."""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["--help"])
    assert result.exit_code == 0
    assert "Robotmk CLI Interface." in result.output
    assert re.search(r"Commands:.*agent.*specialagent.*suite", result.output, re.DOTALL)


def test_cli_invalid_yml_option():
    """No YML config file possible without a context."""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["--yml", "invalid_file"])
    assert result.exit_code == 2
    assert "No such option: --yml" in result.output


def test_cli_invalid_vars_option():
    """No vars file possible without a context."""
    runner = CliRunner()
    result = runner.invoke(cli.main, ["--vars", "invalid_file"])
    assert result.exit_code == 2
    assert "No such option: --vars" in result.output
