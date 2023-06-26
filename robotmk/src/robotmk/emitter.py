import time, sys, os
import json
from copy import deepcopy
from collections import defaultdict
from robotmk.main import Robotmk
from robotmk.context.suite.target.abstract import Target

from robotmk.context.suite.target.target_factory import TargetFactory

# from tabulate import tabulate


class Emitter:
    def __init__(self, config, *args, **kwargs):
        self.config = config

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
        added_settings = {
            "common.context": "suite",
            "common.suiteuname": suiteuname,
        }
        # run_env = basic config + added settings
        self.config.cfg_to_environment(self.config.configdict, environ=run_env)
        self.config.dotcfg_to_env(added_settings, environ=run_env)
        return run_env

    def run(self):
        """Iterates over all suites and produces agent output"""

        suites = self.config.get("suites")
        results = RMKResults()
        for suiteuname, suitecfg in suites:
            # TODO: handle logging
            self.config.set("common.suiteuname", suiteuname)
            target_ = TargetFactory(suiteuname, self.config, None).create()
            results.add(target_)
        print(results.get_results())


class RMKResults:
    def __init__(self):
        self.result_dict = defaultdict(list)

    def add(self, target: Target):
        self.result_dict[target.piggybackhost].append(target.output())

    def get_results(self):
        out = []
        host_results_copies = []
        robotmk_section_header = "<<<robotmk:sep(0)>>>"
        for (
            host,
            results,
        ) in self.result_dict.items():
            host_boundary = {
                True: [f"<<<<{host}>>>>", "<<<<>>>>"],
                False: ["", ""],
            }
            is_piggyback = host != "localhost"
            if is_piggyback:
                host_boundary = [f"<<<<{host}>>>>", "<<<<>>>>"]
            else:
                host_boundary = ["", ""]
            host_results = []
            for result in results:
                host_results.append(json.dumps(result, sort_keys=False, indent=4))
                if is_piggyback:
                    result_short = deepcopy(result)
                    del result_short["output"]
                    host_results_copies.append(result_short)

            host_results[0:0] = [robotmk_section_header]
            host_boundary[1:1] = host_results

            out += host_boundary
        if host_results_copies:  # if there are piggyback results
            out.append(robotmk_section_header)
            out += [
                json.dumps(result, sort_keys=False, indent=4)
                for result in host_results_copies
            ]
        return "\n".join(out)
