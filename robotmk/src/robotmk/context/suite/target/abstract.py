from abc import ABC, abstractmethod
from pathlib import Path
import json
from ..strategies import RunStrategy, RunStrategyFactory
from uuid import uuid4

from robotmk.logger import RobotmkLogger


class Target(ABC):
    """A Target defines the environment where a suite gets executed.

    It's the abstraction of either
    - a local Robot suite or ("target: local")
    - an API call to an external platform ("target: remote") like Robocorp or Kubernetes
    """

    def __init__(self, suiteuname: str, config, logger: RobotmkLogger):
        self.suiteuname = suiteuname
        self.config = config

        self.commoncfg = self.config.get("common")

        self._logger = logger
        if not self._logger is None:
            self.debug = self._logger.debug
            self.info = self._logger.info
            self.warning = self._logger.warning
            self.error = self._logger.error
            self.critical = self._logger.critical

    @property
    def piggybackhost(self):
        return self.config.get("suitecfg.piggybackhost", "localhost")

    @abstractmethod
    def run(self):
        """Abstract method to run a suite/target."""
        pass

    @abstractmethod
    def output(self):
        """Abstract method to get the output of a suite/target."""
        pass
