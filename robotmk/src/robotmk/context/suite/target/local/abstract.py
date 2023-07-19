# mypy: disable-error-code="import, var-annotated"
import json
from abc import ABC, abstractmethod
from pathlib import Path
from uuid import uuid4

from robotmk.config.config import Config
from robotmk.logger import RobotmkLogger

from ...strategies import create_runstrategy
from ..abstract import Target


def _create_suite_path(config: Config) -> Path:
    robotdir = config.get("common.robotdir")
    if robotdir is None:
        raise NotImplementedError("Implementation error.")
    relative_suit_path = config.get("suitecfg.path")
    if relative_suit_path is None:
        raise NotImplementedError("Implementation error.")
    return Path(robotdir).joinpath(relative_suit_path)


class LocalTarget(Target):
    """A FS target is a single RF suite or a RCC task, ready to run from the local filesystem.

    It also encapsulates the implementation details of the RUN strategy, which is
    either a headless or a headed execution (RDP, XVFB, Scheduled Task)."""

    def __init__(
        self,
        suiteuname: str,
        config: Config,
        logger: RobotmkLogger,
    ):
        super().__init__(suiteuname, config.get("suitecfg.piggybackhost", "localhost"))
        self.config = config
        self.logger = logger
        self.path = _create_suite_path(config)
        # TODO: run strategy should not be set in init, because output() always reads results from filesystem
        self.run_strategy = create_runstrategy(self)
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
