from abc import ABC, abstractmethod
import platform
from robotmk.logger import RobotmkLogger
import subprocess
import os
from dataclasses import dataclass, field, asdict
from typing import List

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

    def __init__(self, target) -> None:
        super().__init__(target)

    @abstractmethod
    def exec_pre(self, *args, **kwargs) -> int:
        pass

    @abstractmethod
    def exec_main(self, *args, **kwargs) -> int:
        pass

    @abstractmethod
    def exec_post(self, *args, **kwargs) -> int:
        pass


class WindowsSingleDesktop(WindowsTask):
    """Concrete class to run a suite with UI on Windows.

    ....
    """

    def __init__(self, target) -> None:
        super().__init__(target)

    def exec_pre(self, *args, **kwargs) -> int:
        # create the scheduled task for the given user
        pass

    def exec_main(self, *args, **kwargs) -> int:
        # run schtask.exe to run the task
        pass

    def exec_post(self, *args, **kwargs) -> int:
        pass


class WindowsMultiDesktop(WindowsTask):
    """Concrete class to run a suite in a loopback RDP session.

    This will require a Windows Server with RDP enabled and a proper
    MSTC license. Although there is https://github.com/stascorp/rdpwrap
    (https://www.anyviewer.com/how-to/windows-10-pro-remote-desktop-multiple-users-0427.html)
    """

    def __init__(self, target) -> None:
        super().__init__(target)

    def exec_pre(self, *args, **kwargs) -> int:
        # create RDP file:
        # rdp_file = "loopback.rdp"
        # with open(rdp_file, "w") as f:
        #     f.write(f"""\
        # username:s:{username}
        # password 51:b:{password}
        # full address:s:127.0.0.2
        # """)
        pass

    def exec_main(self, *args, **kwargs) -> int:
        # Launch the RDP session with the specified command
        # os.system(f"mstsc /v:127.0.0.2 /f /w:800 /h:600 /v:127.0.0.2 /u:{username} /p:{password} /v:{rdp_file} /start:{command}")
        # os.system(f'mstsc /v:127.0.0.1 /f /w:800 /h:600 /u:{username} /p:{password} /v:127.0.0.1 /w:800 /h:600 /v:127.0.0.1 /w:800 /h:600 /admin /restrictedAdmin cmd /c "{command}"')

        pass

    def exec_post(self, *args, **kwargs) -> int:
        # Close the RDP session
        # os.system(f'tscon /dest:console')
        pass


class LinuxMultiDesktop(RunStrategy):
    """Executes a suite with a user interface on Linux."""

    def __init__(self, target) -> None:
        super().__init__(target)

    def exec_pre(self, *args, **kwargs) -> int:
        pass

    def exec_main(self, *args, **kwargs) -> int:
        pass

    def exec_post(self, *args, **kwargs) -> int:
        pass


#    __           _
#   / _|         | |
#  | |_ __ _  ___| |_ ___  _ __ _   _
#  |  _/ _` |/ __| __/ _ \| '__| | | |
#  | || (_| | (__| || (_) | |  | |_| |
#  |_| \__,_|\___|\__\___/|_|   \__, |
#                                __/ |
#                               |___/


class RunStrategyFactory:
    """Factory for creating the proper run strategy for a given suite/OS."""

    def __init__(self, target):
        self.target = target

    def create(self) -> RunStrategy:
        """Creates a run strategy based on the given parameters.

        Returns:
            RunStrategy: The run strategy to use.
        """
        mode = self.target.config.get(
            "suites.%s.run.mode" % self.target.config.get("common.suiteuname")
        )
        _platform = platform.system().lower()
        if mode == "default":
            return Runner(self.target)
        elif mode == "windows-1desktop" and _platform == "windows":
            return WindowsSingleDesktop(self.target)
        elif mode == "windows-ndesktop" and _platform == "windows":
            return WindowsMultiDesktop(self.target)
        elif mode == "linux-ndesktop" and _platform == "linux":
            return LinuxMultiDesktop(self.target)
        else:
            raise ValueError(
                "Invalid combination of platform (%s) and run mode (%s)."
                % (_platform, mode)
            )
