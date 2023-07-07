"""This module encapsulates everyhting related so a single suite
execution, either against a local target (RCC/RF) or against a remote one (API)."""

# mypy: disable-error-code="import, empty-body"

from uuid import uuid4
from pathlib import Path
from ..abstract import AbstractContext

from robotmk.config import RobotmkConfigSchema
from .target.abstract import Target
from .target.target_factory import TargetFactory


class SuiteContext(AbstractContext):
    def __init__(self):
        super().__init__()
        # the target object
        self._otarget = None
        self._ymlschema = RobotmkConfigSchema

    @property
    def suiteuname(self):
        """suiteuname under "common" sets the suite to start (suitename + tag)"""
        try:
            suiteuname = self.config.get("common.suiteuname")
        except AttributeError:
            pass
            # TODO: What if suite is not found?
        return suiteuname

    @property
    def target(self) -> Target:
        # singleton
        return self._otarget

    def get_target(self) -> Target:
        if not self._otarget:
            self.init_logger()
            self._otarget = TargetFactory(
                self.suiteuname, self.config, self.logger
            ).create()
        return self._otarget

    def load_config(self, defaults, **kwargs) -> None:
        """Load the config for suite context.

        Suite context can merge the config from
        - OS defaults
        - + environment variables
        - + YML file (default/custom = --yml)
        - + var file (= --vars)
        - + environment variables


        """
        if kwargs.get("default_cfg", {}):
            # Suite was started by scheduler, config was passed
            self.config.configdict = kwargs["default_cfg"]
        else:
            # After the defaults are read...
            self.config.set_defaults(defaults)
            # ...lets first search for env vars which point to another location of the robotmk.yml!
            self.config.read_cfg_vars(path=None)
            self.config.read_yml_cfg(path=kwargs["ymlfile"], must_exist=False)
            # In the end, environment variables can override everything
            self.config.read_cfg_vars(path=kwargs["varfile"])

        # TODO: validate later so that config can be dumped
        # self.config.validate(self._ymlschema)

    def refresh_config(self) -> bool:
        """Re-loads the config and returns True if it changed"""
        # TODO: implement this
        pass

    # def run_default(self):
    #     """Implements the default action for suite context."""
    #     # TODO: execute one single suite
    #     print("Suite context default action = execute single suite ")
    #     pass

    def execute(self):
        """Runs a single suite, either fs/remote."""
        # TODO: is it better to pass the suitename to get_target()?
        self.get_target().run()

    def output(self):
        """Gathers the result of the given suite (fs/remote) and returns it"""
        return self.get_target().output()
