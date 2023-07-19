import os
import platform
import subprocess
from abc import ABC, abstractmethod
from collections import UserDict
from dataclasses import asdict, dataclass, field
from typing import List


@dataclass
class Result:
    """Result of a subprocess execution."""

    args: List[str] = field(default_factory=list)
    returncode: int = 0
    stdout: List[str] = field(default_factory=list)
    stderr: List[str] = field(default_factory=list)


@dataclass(frozen=True)
class RunnerCfg:
    pre_command: str | list[str]
    main_command: str | list[str]
    post_command: str | list[str]


class Runner:
    """This Strategy is the only one which executes a 'job' in fact.

    - run a Robot Framework Suite
    - run a RCC task
    """

    def __init__(self, config: RunnerCfg) -> None:
        self.config = config

    def run(self, env: UserDict) -> tuple[int, Result]:
        """Template method which bundles the linked methods to run.

        The concrete strategy selectivly overrides the methods to implement."""
        pre = self.exec_pre()
        main = self.exec_main(env)
        post = self.exec_post()
        return_code = max(pre.returncode, main.returncode, post.returncode)
        return return_code, main

    def run_subprocess(self, command, environ) -> Result:
        """If command was given, run the subprocess and return the result object."""
        if not command:
            return Result()
        res = subprocess.run(command, capture_output=True, env=environ)
        return Result(
            args=res.args,
            returncode=res.returncode,
            stdout=res.stdout.decode("utf-8").splitlines(),
            stderr=res.stderr.decode("utf-8").splitlines(),
        )

    def exec_pre(self) -> Result:
        """Prepares the given suite."""
        return self.run_subprocess(self.config.pre_command, os.environ)

    def exec_main(self, env: UserDict) -> Result:
        """Execute the the given suite."""
        # DEBUG: " ".join(self.config.main_command)

        # DEBUG: [f"{k}={v}" for (k,v) in environment.items()  if k.startswith("RO")]
        # DEBUG: "; ".join([f"export {k}={v}" for (k,v) in kwargs.get("env").items() if k.startswith("RO")])
        # TODO: log console output? Save it anyway because a a fatal RF error must be tracable.
        # RCC does not re.execute...
        return self.run_subprocess(self.config.main_command, env)

    def exec_post(self) -> Result:
        """Cleans up the given suite."""
        return self.run_subprocess(self.config.post_command, os.environ)


class WindowsTask(Runner):
    """Class for Single and Multi desktop strategies.

    Both have in common that they need to create a scheduled task."""


class WindowsSingleDesktop(Runner):
    """Class to run a suite with UI on Windows.

    - Create the scheduled task for the given user.
    - Run the task via schtask.exe
    """


class WindowsMultiDesktop(Runner):
    """Class to run a suite in a loopback RDP session.

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


class LinuxMultiDesktop(Runner):
    """Executes a suite with a user interface on Linux."""


def create_runstrategy(target) -> Runner:
    """Creates a run strategy based on the given suite/OS.

    Returns:
        Runner: The run strategy to use.
    """
    suite_name = target.config.get("common.suiteuname")
    mode = target.config.get(f"suites.{suite_name}.run.mode")
    _platform = platform.system().lower()
    config = RunnerCfg(
        pre_command=target.pre_command,
        main_command=target.main_command,
        post_command=target.post_command,
    )
    if mode == "default":
        return Runner(config)
    if mode == "windows-1desktop" and _platform == "windows":
        raise NotImplementedError("WindowsSingleDesktop")
    if mode == "windows-ndesktop" and _platform == "windows":
        raise NotImplementedError("WindowsMultiDesktop")
    if mode == "linux-ndesktop" and _platform == "linux":
        raise NotImplementedError("LinuxMultiDesktop")
    raise ValueError(
        f"Invalid combination of platform ({_platform}) and run mode ({mode})."
    )
