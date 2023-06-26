import os
import math

from robotmk.logger import RobotmkLogger
from .abstract import LocalTarget
from ...strategies import RunStrategy


# TODO: make RCC binary path configurabe


class RCCTarget(LocalTarget):
    def __init__(
        self,
        suiteuname: str,
        config: dict,
        logger: RobotmkLogger,
    ):
        super().__init__(suiteuname, config, logger)

    @staticmethod
    def worker_count():
        """Determines the number of RCC worker processes by the total CPU cores available.

        Example:
        - 4 cores: 2 workers
        - 5 cores: 2 workers
        - 6 cores: 3 workers
        - 7 cores: 3 workers
        - 8 cores: 4 workers
        """
        return max(min(math.floor(os.cpu_count() / 2), 2), 6)

    def __str__(self) -> str:
        return "rcc"

    @property
    def pre_command(self) -> list:
        """Runs just before the command, executed by the Run strategy"""
        return [
            "rcc",
            "holotree",
            "vars",
            "--controller",
            "robotmk",
            "--space",
            self.suiteuname,
            "--workers",
            str(self.worker_count()),
            "-r",
            str(self.path.joinpath("robot.yaml")),
        ]

    @property
    def main_command(self) -> list:
        """The command will be used by the Run strategy (self.target.command).

        In RCC target, the commandline gets buuilt to execute a RCC task (=Robotmk inside of RCC)
        """
        # DEBUG: "; ".join([f"export {k}={v}" for (k,v) in kwargs.get("env").items() if k.startswith("RO")])
        # paste export strings into shell
        # - which python3 -> shout point to RCC venv
        # - env -> ROBOCORP and ROBOTMK vars should be set
        # - which robotmk -> path to entrypoint
        # python3 -m trace --trace <ENTRYPOINT> suite run suite_default
        return [
            "rcc",
            "task",
            "run",
            "--controller",
            "robotmk",
            "--space",
            self.suiteuname,
            "-t",
            "robotmk",
            "-r",
            str(self.path.joinpath("robot.yaml")),
        ]

    def prepare_environment(self) -> dict:
        """Sets up the environment for a subsequent RCC robot run.

        If Robotmk calls itself in a RCC task, the inner call of Robotmk needs
        some special settings, e.g. NOT to use RCC again, to log into the default
        logdir, etc.
        This function first exports the current config to the environment and
        then adds the special settings on top.
        """
        env = os.environ.copy()
        added_settings = {"suitecfg.run.rcc": False, "suitecfg.uuid": self.uuid}
        self.config.cfg_to_environment(self.config.configdict, environ=env)
        self.config.dotcfg_to_env(added_settings, environ=env)
        return env

    def run(self):
        # Before we blow up the whole thing, we should check if the suite is RCC compatible and allowed to run
        if self.is_disabled_by_flagfile:
            # TODO: Log skipped
            # reason = self.get_disabled_reason()
            return
        if not self.is_rcc_compatible:
            # TODO: log suite not compatible with RCC
            pass
        else:
            run_env = self.prepare_environment()
            self.rc = self.run_strategy.run(env=run_env)

    @property
    def is_rcc_compatible(self):
        """Returns True if the given suite folder is compatible with RCC.
        Such a suite dir must at least contain conda.yml and robot.yml.
        """
        if (
            self.path.joinpath("conda.yaml").exists()
            and self.path.joinpath("robot.yaml").exists()
        ):
            return True
        else:
            return False

    # def calculate_blueprint_hash(self):
    #     try:
    #         output = subprocess.check_output(
    #             ["rcc", "ht", "hash", self.blueprint_path], universal_newlines=True
    #         )
    #         self.blueprint_hash = output.strip()
    #     except subprocess.CalledProcessError as e:
    #         raise RuntimeError(
    #             f"Failed to calculate blueprint hash: {e.stderr.strip()}"
    #         )

    # def is_environment_ready(self):
    #     if not self.blueprint_hash:
    #         self.calculate_blueprint_hash()
    #     try:
    #         output = subprocess.check_output(
    #             ["rcc", "ht", "spaces", "--filter", self.blueprint_hash],
    #             universal_newlines=True,
    #         )
    #         spaces = json.loads(output)
    #         return bool(spaces)
    #     except subprocess.CalledProcessError as e:
    #         raise RuntimeError(
    #             f"Failed to check environment readiness: {e.stderr.strip()}"
    #         )

    # def check_spaces(self):
    #     if not self.blueprint_hash:
    #         self.calculate_blueprint_hash()
    #     try:
    #         output = subprocess.check_output(
    #             ["rcc", "ht", "spaces", "--filter", self.blueprint_hash],
    #             universal_newlines=True,
    #         )
    #         spaces = json.loads(output)
    #         return spaces
    #     except subprocess.CalledProcessError as e:
    #         raise RuntimeError(f"Failed to check spaces: {e.stderr.strip()}")

    # def create_environment(self, name, variables=None):
    #     if not self.blueprint_hash:
    #         self.calculate_blueprint_hash()
    #     cmd = ["rcc", "ht", "vars", "--blueprint", self.blueprint_hash, "--name", name]
    #     if variables:
    #         cmd.extend(["--vars", json.dumps(variables)])
    #     try:
    #         subprocess.check_call(cmd)
    #     except subprocess.CalledProcessError as e:
    #         raise RuntimeError(f"Failed to create environment: {e.stderr.strip()}")
