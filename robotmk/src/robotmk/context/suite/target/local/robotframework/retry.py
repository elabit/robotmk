# mypy: disable-error-code="import"
import glob
import os
from abc import ABC, abstractmethod
from dataclasses import asdict
from pathlib import Path

from robot.rebot import rebot

from robotmk.context.suite.target.local.abstract import LocalTarget


def create_retry_strategy(target: LocalTarget):
    """Create the execution strategy"""
    strategy = target.config.get("suitecfg.retry_failed.strategy", "complete")
    if strategy == "complete":
        return CompleteRetry(target)
    if strategy == "incremental":
        return IncrementalRetry(target)
    raise NotImplementedError(f"Unknown retry strategy: {strategy}")


class RetryStrategy(ABC):
    """Execution strategy interface for suites"""

    def __init__(self, target):
        self.target = target
        self.target.attempt = 1

    @property
    def max_attempts(self):
        """Maximum number of attempts to execute a suite (1st + retries)"""
        return 1 + self.target.config.get("suitecfg.retry_failed.retry_attempts", 0)

    def run(self):
        """Run the suite and retry failed tests if necessary."""

        # TODO: set TIC

        for attempt in range(1, self.max_attempts + 1):
            # if self.max_attempts > 1:
            #     self._runner.loginfo(
            #         f" > Starting attempt {attempt}/max {max_exec} ({str(self.})"
            #     )
            # else:
            #     self._runner.loginfo(f" > Starting suite...")
            self.target.attempt = attempt

            # TODO: log the cli args
            (return_code, result) = self.target.run_strategy.run(os.environ)
            self.target.console_results[self.target.attempt] = asdict(result)
            # TODO: Logging
            # if rc > 250:
            #     self.logerror(
            #         "RC > 250 = Robot exited with fatal error. There are no logs written."
            #     )
            if self.max_attempts == 1 or (
                self.target.attempt == 1 and return_code == 0
            ):
                # if only one attempt allowed or 1st attempt was OK, we are done
                break
            else:
                # more attempts allowed and 1st attempt was not OK
                if return_code == 0:
                    # this retry was OK, get out here
                    self._finalize_results()
                    break
                else:
                    if self.target.attempt < self.max_attempts:
                        self._reparametrize()
                    else:
                        # ...GAME OVER! => MERGE
                        # TODO: logging
                        # self._runner.loginfo(
                        #     "   Even the last attempt was unsuccessful!"
                        # )
                        self._finalize_results()
        return return_code

    @abstractmethod
    def _reparametrize(self):
        """Reparametrize the suite for the next attempt.

        Only incremental strategy needs to do something here."""

    def _finalize_results(self):
        """Merge the XML result files into a new final result"""

        # Attempt "None" sets output filenames without a attempt number
        self.target.attempt = None

        outputfiles = self._glob_target_outputfiles()
        filenames = [Path(f).name for f in outputfiles]
        # TODO: log the files to merge
        # TODO: use string fields of the subclasses for individual logging
        # for f in filenames:
        #     self.suite._runner.logdebug(" - %s" % f)

        # rebot wants to print out the generated file names on stdout; write to devnull
        devnull = open(os.devnull, "w")
        rebot(
            *outputfiles,
            outputdir=self.target.outputdir,
            output=self.target.output_xml,
            log=self.target.log_html,
            report=None,
            merge=True,
            stdout=devnull,
        )
        # self.suite._runner.loginfo("Merged results of all reexecutions into:")
        # self.suite._runner.loginfo(" - %s" % self.suite.output)
        # self.suite._runner.loginfo(" - %s" % self.suite.log)

    def _glob_target_outputfiles(self):
        """Returns the list of XML output files of the target execution attempts 1..n"""
        output_filename = self.target.output_filename.rsplit("-", 1)[0]
        glob_pattern = f"{output_filename}-*.xml"
        outputfiles = sorted(
            glob.glob(str(Path(self.target.outputdir).joinpath(glob_pattern)))
        )
        return outputfiles


class CompleteRetry(RetryStrategy):
    """Execution strategy for suites with complete re-execution"""

    def __str__(self):
        return "Strategy: Complete"

    def _reparametrize(self):
        """Reparametrize the suite for the next attempt.

        Only incremental strategy needs to do something here."""


class IncrementalRetry(RetryStrategy):
    """Provides methods to re-execute suites incrementally (no test interdependency)"""

    def __str__(self):
        return "Strategy: Incremental"

    def _reparametrize(self):
        """Reparametrize the suite for the next attempt.

        The next attempt needs the XML file of the last attempt as input.
        From there it will read failed tests and re-execute them only."""
        # Chance for next try. Attempt gets increased, output files get bumped
        failed_xml = Path(self.target.outputdir).joinpath(self.target.output_xml)
        self.target.robot_params.update({"rerunfailed": str(failed_xml)})
        rerun_selection = self.target.config.get(
            "suitecfg.retry_failed.rerun_selection", asdict=True, default={}
        )
        self.target.robot_params.update(rerun_selection)
