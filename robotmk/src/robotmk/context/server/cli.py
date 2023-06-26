"""Run the log viewer web server.

Starts a local webserver to serve the execution logs of all Robot suites."""
import sys
import click
from robotmk.main import Robotmk, DEFAULTS


# # use module docstring as help text
# @click.command(help=__doc__)
# @click.pass_context
# def server(ctx):
#     click.secho("server", fg="green")
#     click.echo("Serving the Robotmk webinterface on http://localhost:8099 ...")
#     # ctx.robotmk = Robotmk("server")
#     # ctx.obj.config.set("common.context", "server")
#     pass
