from abc import ABC, abstractmethod
from robotmk.config import Config
from robotmk.logger import RobotmkLogger
from pathlib import Path


class AbstractContext(ABC):
    """Abstract class for context objects. Context objects are used to
    encapsulate the different contexts in which robotmk can be run (local, specialagent, suite).
    """

    def __init__(self):
        # self._config_factory = Config()
        self._logger = None
        self.init_logger()
        self.config = Config()

    @property
    def logger(self):
        # self.__init_logger()
        return self._logger

    def init_logger(self):
        # initialize the logger only when config was loaded
        if self._logger is None and getattr(self, "config", None):
            self._logger = RobotmkLogger(
                Path(self.config.get("common.logdir")).joinpath("robotmk.log"),
                self.config.get("common.log_level"),
            )
            self.debug = self._logger.debug
            self.info = self._logger.info
            self.warning = self._logger.warning
            self.error = self._logger.error
            self.critical = self._logger.critical

    @abstractmethod
    def load_config(self, defaults, **kwargs) -> None:
        """Depening on the context strategy, the config object loads cfg from different sources."""
        raise NotImplementedError("Subclass must implement abstract method")

    @abstractmethod
    def refresh_config(self):
        """Load the config again, e.g. after a change in the config file."""
        raise NotImplementedError("Subclass must implement abstract method")

    # @abstractmethod
    # def run_default(self):
    #     """Encapsulates everything that needs to be done to
    #     run robotmk when it is run only with context, but without subcommand."""
    #     raise NotImplementedError("Subclass must implement abstract method")

    @abstractmethod
    def execute(self, *args, **kwargs):
        """Encapsulates everything that needs to be done to run robotmk."""
        raise NotImplementedError("Subclass must implement abstract method")

    @abstractmethod
    def output(self):
        """Encapsulates everything that needs to be done to produce agent output."""
        raise NotImplementedError("Subclass must implement abstract method")
