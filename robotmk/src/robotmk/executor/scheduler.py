# mypy: disable-error-code="import"
import atexit
import os
import subprocess
import sys
import time
from pathlib import Path

# from apscheduler.schedulers.blocking import BlockingScheduler
from apscheduler.executors.pool import ProcessPoolExecutor
from apscheduler.schedulers.background import BackgroundScheduler
from tabulate import tabulate

from robotmk.main import Robotmk

from .abstract import AbstractExecutor


class Scheduler(AbstractExecutor):
    def __init__(self, config, foreground=False, max_deadman_file_age=300):
        # TODO: Max. number of prcesses must be lower than the number of CPUs
        super().__init__(config)
        tmpdir = Path(self.config.get("common.tmpdir"))
        # Ref 7e8b2c1 (controller plugin)
        self.pidfile = tmpdir / "robotmk_agent.pid"
        self.foreground = foreground
        # max age of the controller deadman file in seconds before the agent
        # stops running
        self.max_deadman_file_age = max_deadman_file_age
        self.rmk_ctrl_deadman_file = tmpdir / "robotmk_controller_deadman_file"
        self.scheduler = BackgroundScheduler(
            executors={"mydefault": ProcessPoolExecutor(6)}
        )
        # where the agent can signal the controller the reason for exiting
        self.last_agent_exitmsg_file = tmpdir / "robotmk_agent_lastexitcode"

        self.suitecfg_hashes = {}
        # self.scheduler = BlockingScheduler(
        #     executors={"mydefault": ProcessPoolExecutor(6)}
        # )

    @property
    def pid(self):
        return os.getpid()

    def start_robotmk_process(self, run_env):
        cmd = "robotmk suite run".split(" ")
        result = subprocess.run(cmd, capture_output=True, env=run_env)
        # result = subprocess.run(["echo", "foo"], capture_output=True, env=run_env)
        stdout_str = result.stdout.decode("utf-8").splitlines()
        stderr_str = result.stderr.decode("utf-8").splitlines()
        result_dict = {
            "args": result.args,
            "returncode": result.returncode,
            "stdout": stdout_str,
            "stderr": stderr_str,
        }
        pass

    def prepare_environment(self, suiteuname) -> dict:
        run_env = os.environ.copy()
        self.config.cfg_to_environment(self.config.configdict, environ=run_env)
        # add suite specific settings to the environment
        self.config.dotcfg_to_env(
            {
                "common.context": "suite",
                "common.suiteuname": suiteuname,
            },
            environ=run_env,
        )
        return run_env

    def schedule_jobs(self):
        """Updates the scheduler with new jobs and removes old ones"""
        suites = self.config.get("suites")
        current_jobs = set(self.scheduler.get_jobs())
        # remove jobs that are no longer in the config
        for job in current_jobs - set(suites.asdict().keys()):
            self.scheduler.remove_job(job.id)
        # schedule new/changed jobs
        for suiteuname, suitecfg in suites:
            hash = self.config.suite_cfghash(suiteuname)
            if hash == self.suitecfg_hashes.get(suiteuname, {}):
                # The suite config has not changed, so we can skip it
                continue
            else:
                run_env = self.prepare_environment(suiteuname)
                interval = suitecfg.get("scheduling.interval")

                self.scheduler.add_job(
                    self.start_robotmk_process,
                    args=[run_env],
                    trigger="interval",
                    id=suiteuname,
                    seconds=interval,
                    replace_existing=True,
                    # args=[v],
                    max_instances=1,
                )
                self.suitecfg_hashes[suiteuname] = hash

    def run(self):
        """Start the scheduler and update the jobs every 5 seconds"""

        self.schedule_jobs()
        # self.scheduler.add_listener(self.log)
        self.scheduler.start()
        # HEREIWAS
        while self.running_allowed():
            self.touch_pidfile()
            # update the jobs every 5 seconds
            time.sleep(5)

            jobs = self.scheduler.get_jobs(jobstore=None)
            table = []
            for job in jobs:
                table.append(
                    [
                        job.id,
                        job.name,
                        # job.args,
                        job.trigger,
                        job.pending,
                        job.next_run_time,
                        job.trigger.interval.total_seconds(),
                    ]
                )
            # print current time
            print(time.strftime("%H:%M:%S", time.localtime()))
            print(
                tabulate(
                    table,
                    headers=[
                        "id",
                        "name",
                        "trigger",
                        "pending",
                        "next_run at",
                        "interval",
                    ],
                )
            )
            print("\n")

        # This point is reached only when the while loop is exited and the
        # agent is not allowed to run anymore
        self.unlink_pidfile()
        self.exit_with_filecode(
            202,
            "Robotmk Agent exited, Reason: missing/outdated controller deadman file %s"
            % str(self.rmk_ctrl_deadman_file),
        )

    def running_allowed(self):
        if self.foreground:
            # ignore the deadman switch and run anyway
            return True
        else:
            # let the deadman switch decide if we are allowed to run
            return self.ctrl_deadman_file_is_fresh()

    def ctrl_deadman_file_is_fresh(self):
        # if exists
        if not self.rmk_ctrl_deadman_file.exists():
            return False
        else:
            mtime = os.path.getmtime(self.rmk_ctrl_deadman_file)
            now = time.time()
            if now - mtime < self.max_deadman_file_age:
                return True
            else:
                return False

    def touch_pidfile(self):
        try:
            with open(self.pidfile, "w+", encoding="ascii") as f:
                f.write(str(self.pid) + "\n")
        except IOError:
            print(__name__ + ": " + "Could not write PID file %s" % self.pidfile)
            sys.exit(1)
        # TODO: deletes the pidfile on exit, really....?
        atexit.register(self.unlink_pidfile)

    def unlink_pidfile(self):
        """Deletes the PID file"""
        if os.path.exists(self.pidfile):
            os.remove(self.pidfile)

    def exit_with_filecode(self, code, message=""):
        # Writes the exit code and message to a file, so that the controller
        # can read it, and exits
        with open(self.last_agent_exitmsg_file, "w") as f:
            f.write(f"{str(code)} {message}")
        sys.exit(int(code))
