from abc import ABC, abstractmethod
from pathlib import Path
from uuid import uuid4
import json

from ..abstract import Target
from ...strategies import RunStrategyFactory
from robotmk.logger import RobotmkLogger


class LocalTarget(Target):
    """A FS target is a single RF suite or a RCC task, ready to run from the local filesystem.

    It also encapsulates the implementation details of the RUN strategy, which is
    either a headless or a headed execution (RDP, XVFB, Scheduled Task)."""

    def __init__(
        self,
        suiteuname: str,
        config: dict,
        logger: RobotmkLogger,
    ):
        super().__init__(suiteuname, config, logger)
        self.path = Path(self.config.get("common.robotdir")).joinpath(
            self.config.get("suitecfg.path")
        )
        # TODO: run strategy should not be set in init, because output() always reads results from filesystem
        self.run_strategy = RunStrategyFactory(self).create()
        # list of subprocess' results and console output
        self.console_results = {}

    @abstractmethod
    def run(self):
        """Implementation in subclasses RCCTarget and RobotFrameworkTarget"""
        pass

    def output(self):
        """Read the result artifacts from the filesystem."""
        try:
            with open(self.statefile_fullpath) as f:
                data = json.load(f)
                return data
        except FileNotFoundError:
            # return an empty dict to indicate that the file is not present
            return {}

    @property
    def pre_command(self):
        return None

    @property
    def main_command(self):
        return None

    @property
    def post_command(self):
        return None

    @property
    def uuid(self):
        """If a UUID is already part of the suite config, use this. Otherwise generate a new one.

        The idea is that the UUID is handed over between all robotmk calls and lastly part of the
        result JSON."""
        uuid_ = self.config.get("suitecfg.uuid", False)
        if not uuid_:
            uuid_ = uuid4().hex
        return uuid_

    @property
    def logdir(self):
        return self.config.get("common.logdir")

    @property
    def resultdir(self):
        return Path(self.logdir).joinpath("results")

    @property
    def statefile_fullpath(self):
        return str(Path(self.resultdir).joinpath(self.suiteuname + ".json"))

    @property
    def is_disabled_by_flagfile(self):
        """The presence of a file DISABLED inside of a Robot suite will prevent
        Robotmk to execute the suite, either by RCC or RobotFramework."""
        return self.path.joinpath("DISABLED").exists()
