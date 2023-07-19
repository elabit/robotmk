# mypy: disable-error-code="import, empty-body"
import os
import platform
import subprocess
from abc import ABC, abstractmethod
from dataclasses import asdict, dataclass, field
from typing import List

from robotmk.logger import RobotmkLogger

# TODO: split this into modules
# TODO:


@dataclass
class Result:
    """Result of a subprocess execution."""

    args: List[str] = field(default_factory=list)
    returncode: int = 0
    stdout: List[str] = field(default_factory=list)
    stderr: List[str] = field(default_factory=list)


class RunStrategy(ABC):
    def __init__(self, target) -> None:
        self.target = target

        # self.suiteuname = suiteuname
        # self.config = config
        # self._logger = logger
        # self.debug = self._logger.debug
        # self.info = self._logger.info
        # self.warning = self._logger.warning
        # self.error = self._logger.error
        # self.critical = self._logger.critical

    def run(self, *args, **kwargs):
        """Template method which bundles the linked methods to run.

        The concrete strategy selectivly overrides the methods to implement."""
        rc = max(
            self.exec_pre(*args, **kwargs),
            self.exec_main(*args, **kwargs),
            self.exec_post(*args, **kwargs),
        )
        return rc

    @abstractmethod
    def exec_pre(self, *args, **kwargs) -> int:
        """Prepares the given suite."""
        pass

    @abstractmethod
    def exec_main(self, *args, **kwargs) -> int:
        """Execute the the given suite."""
        pass

    @abstractmethod
    def exec_post(self, *args, **kwargs) -> int:
        """Cleans up the given suite."""
        pass


class Runner(RunStrategy):
    """This Strategy is the only one which executes a 'job' in fact.

    - run a Robot Framework Suite
    - run a RCC task
    """

    def __init__(self, target) -> None:
        super().__init__(target)

    def run_subprocess(self, command, environ) -> Result:
        """If command was given, run the subprocess and return the result object."""
        if command:
            res = subprocess.run(command, capture_output=True, env=environ)
            return Result(
                args=res.args,
                returncode=res.returncode,
                stdout=res.stdout.decode("utf-8").splitlines(),
                stderr=res.stderr.decode("utf-8").splitlines(),
            )
        else:
            return Result()

    def exec_pre(self, *args, **kwargs) -> int:
        result = self.run_subprocess(self.target.pre_command, os.environ)
        return result.returncode

    def exec_main(self, *args, **kwargs) -> int:
        # DEBUG: " ".join(self.target.main_command)

        # DEBUG: [f"{k}={v}" for (k,v) in environment.items()  if k.startswith("RO")]
        # DEBUG: "; ".join([f"export {k}={v}" for (k,v) in kwargs.get("env").items() if k.startswith("RO")])
        result = self.run_subprocess(
            self.target.main_command, kwargs.get("env", os.environ)
        )

        # TODO: log console output? Save it anyway because a a fatal RF error must be tracable.
        # RCC does not re.execute...
        if getattr(self.target, "attempt", None) is None:
            self.target.attempt = 1
        self.target.console_results[self.target.attempt] = asdict(result)
        return result.returncode

    def exec_post(self, *args, **kwargs) -> int:
        result = self.run_subprocess(self.target.post_command, os.environ)
        return result.returncode


class WindowsTask(RunStrategy):
    """Parent class for Single and Multi desktop strategies.

    Both have in common that they need to create a scheduled task."""


class WindowsSingleDesktop(WindowsTask):
    """Concrete class to run a suite with UI on Windows.

    - Create the scheduled task for the given user.
    - Run the task via schtask.exe
    """


class WindowsMultiDesktop(WindowsTask):
    """Concrete class to run a suite in a loopback RDP session.

    This will require a Windows Server with RDP enabled and a proper
    MSTC license. Although there is https://github.com/stascorp/rdpwrap
    (https://www.anyviewer.com/how-to/windows-10-pro-remote-desktop-multiple-users-0427.html)

    The following steps are required:

    - Create a RDP file:
      ```
      loopback.rdp
      username:s:{username}
      password 51:b:{password}
      full address:s:127.0.0.2
      ```
    - Launch the RDP session with a specified `command`:
      $ mstsc /v:127.0.0.2 /f /w:800 /h:600 /u:{username} /p:{password} /v:{rdp_file} /start:{command}
      $ mstsc /v:127.0.0.1 /f /w:800 /h:600 /u:{username} /p:{password} /admin /restrictedAdmin cmd /c "{command}
    - Close the RDP session:
      $ tscon /dest:console
    """


class LinuxMultiDesktop(RunStrategy):
    """Executes a suite with a user interface on Linux."""


def create_runstrategy(target) -> RunStrategy:
    """Creates a run strategy based on the given suite/OS.

    Returns:
        RunStrategy: The run strategy to use.
    """
    mode = target.config.get(
        "suites.%s.run.mode" % target.config.get("common.suiteuname")
    )
    _platform = platform.system().lower()
    if mode == "default":
        return Runner(target)
    if mode == "windows-1desktop" and _platform == "windows":
        raise NotImplementedError("WindowsSingleDesktop")
    if mode == "windows-ndesktop" and _platform == "windows":
        raise NotImplementedError("WindowsMultiDesktop")
    if mode == "linux-ndesktop" and _platform == "linux":
        raise NotImplementedError("LinuxMultiDesktop")
    raise ValueError(
        "Invalid combination of platform (%s) and run mode (%s)." % (_platform, mode)
    )
