from pathlib import Path

from ..abstract import AbstractContext

from robotmk.config.yml import RobotmkConfigSchema
from robotmk.executor.sequencer import Sequencer


class SpecialAgentContext(AbstractContext):
    def __init__(self):
        super().__init__()
        self.ymlschema = RobotmkConfigSchema

    @property
    def logger(self):
        if not self._logger:
            self._logger = RobotmkLogger(
                Path(self.config.common.logdir).joinpath("robotmk.log"),
                self.config.common.log_level,
            )
        return self._logger

    def load_config(self, defaults, **kwargs) -> None:
        """Load the config for specialagent context.

        This context can merge the config from
        - OS defaults
        - + var file (= --vars)
        - + environment variables

        (There is no YML file for specialagent context!)
        """
        self.config.set_defaults(defaults)
        self.config.read_cfg_vars(path=kwargs.get("path"))
        # TODO: validate later so that config can be dumped
        # self.config.validate(self.ymlschema)

    def refresh_config(self) -> bool:
        """Re-loads the config and returns True if it changed"""
        # TODO: implement this

    # def run_default(self):
    #     """Implements the default action for specialagent context."""
    #     # TODO: start the sequencer
    #     print("Specialagent context default action = trigger APIs and output")
    #     pass

    def execute(self, *args, **kwargs):
        """Implements the run action for specialagent context."""
        print("Specialagent context run action")
        self.executor = Sequencer(self.config)
        self.executor.run()

    def output(self):
        """Implements the agent output for local context."""
        print("Specialagent context output")
        self.outputter = SpecialAgentOutput(self.config)
        pass
