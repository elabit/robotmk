# mypy: disable-error-code="import"
"""CLI commands for execution of a single suite."""

import sys
import click
import json
from robotmk.cli.defaultgroup import DefaultGroup
from robotmk.main import Robotmk, DEFAULTS

# TODO: Refine the defaultgroup usage

#             _ _
#            (_) |
#   ___ _   _ _| |_ ___
#  / __| | | | | __/ _ \
#  \__ \ |_| | | ||  __/
#  |___/\__,_|_|\__\___|


# use module docstring as help text
@click.group(
    cls=DefaultGroup, default_if_no_args=True, help=__doc__, invoke_without_command=True
)
@click.pass_context
@click.option("--yml", "-y", help="Read config from custom YML file")
@click.option("--vars", "-v", help="Read vars from .env file (ignores environment)")
def suite(ctx, yml, vars):
    if vars and yml:
        raise click.BadParameter("Cannot use --yml and --vars at the same time")
    ctx_loglevel = ctx.parent.params.get("loglevel", DEFAULTS["common"]["log_level"])
    ctx.obj = Robotmk(
        contextname="suite", log_level=ctx_loglevel, ymlfile=yml, varfile=vars
    )


@suite.command(default=True)
@click.argument("suite", required=False)
@click.pass_context
def run(ctx, suite):
    """Trigger the start of a Robot Framework SUITE.

    SUITE must be a configuration subkey of the "suites" section.
    (can also be set by env:ROBOTMK_common_suiteuname.)
    """
    if suite:
        # suite was given as argment on CLI
        ctx.obj.config.set("common.suiteuname", suite)
    if bool(ctx.obj.config.get("common.suiteuname", None)):
        # suite was set in environment
        ctx.obj.execute()
    else:
        click.secho("Suite '%s' not found in configuration" % suite, fg="red")


@suite.command()
@click.argument("suite", required=False)
@click.pass_context
def result(ctx, suite):
    """Print the result JSON of a SUITE on STDOUT.

    SUITE must be a configuration subkey of the "suites" section.
    (can also be set by env:ROBOTMK_common_suiteuname.)

    To get the Checkmk Agent output, use "agent output" instead.
    """
    if suite:
        ctx.obj.config.set("common.suiteuname", suite)
    if bool(ctx.obj.config.get("common.suiteuname", None)):
        data = ctx.obj.output()
        click.secho(json.dumps(data, indent=4), fg="bright_white")

    else:
        click.secho("Suite '%s' not found in configuration" % suite, fg="red")


@suite.command()
@click.argument("suite", required=True)
@click.option(
    "--number",
    "-n",
    help="Number of last execution logs of SUITE to show",
    default=1,
    show_default=True,
)
@click.option(
    "--pid", "-p", help="Shows the execution log of SUITE with a specific PID"
)
@click.pass_context
def logs(ctx, suite, number, pid):
    """Display the log files of SUITE.

    SUITEID is eiher equal to the suite dir or a combination of suite dir and its unique tag as set in the configuration.
    Examples are: suite1, suite2_tagfoo, suite3_tagbaz
    (can also be set by env:ROBOTMK_common_suiteuname.)
    """
    click.secho("logs", fg="green")
    if int(number) != 1 and pid != None:
        raise click.BadParameter("Cannot use --number and --pid at the same time %d")
    click.echo("These are the logs of suite %s %d:" % (suite, number))
    pass


# TODO: implement this
@suite.command()
@click.argument("suite", required=False)
@click.pass_context
def shell(ctx, suite):
    """Open a shell in the RCC python environment of SUITE.

    SUITEID is eiher equal to the suite dir or a combination of suite dir and its unique tag as set in the configuration.
    Examples are: suite1, suite2_tagfoo, suite3_tagbaz
    (can also be set by env:ROBOTMK_common_suiteuname.)
    """
    click.secho("$> RCC shell of suite %s" % suite, fg="yellow")

    pass


@suite.command()
@click.pass_context
def vardump(ctx):
    click.secho("vardump", fg="green")
    # TODO: implement this
    pass


@suite.command(help="Dump the config as YML to STDOUT or FILE")
# add file arg
@click.argument("file", required=False, type=click.Path(exists=False))
@click.pass_context
def ymldump(ctx, file):
    click.secho(ctx.obj.config.to_yml(file), fg="bright_white")
    sys.exit(0)
