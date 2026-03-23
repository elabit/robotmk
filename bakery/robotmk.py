#!/usr/bin/env python3
# -*- coding: utf-8 -*-

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

ROBOTMK_VERSION = '1.6.0'

from typing import Iterable, TypedDict, List
from pathlib import Path
import os
import yaml
import re
import copy

from cmk.base.cee.plugins.bakery.bakery_api.v1 import (
    OS,
    Plugin,
    PluginConfig,
    WindowsConfigEntry,
    register,
    FileGenerator,
    SystemBinary,
)

try:
    # Checkmk 2.4+
    from cmk.ccc.exceptions import MKGeneralException
except ImportError:
    # Checkmk <= 2.3
    from cmk.utils.exceptions import MKGeneralException

# This dict only adds the new key only if
# * the key already exists
# * the value is a boolean in fact
# * the value contains something meaningful
# This prevents that empty dicts are set as values.
class DictNoNone(dict):
    def __setitem__(self, key, value):
        if (key in self or type(value) is bool) or bool(value):
            dict.__setitem__(self, key, value)


# This class is common with CMK 1/2
class RMKSuite:
    def __init__(self, suite_dict):
        self.suite_dict = suite_dict

    @property
    def suite2dict(self):
        suite_dict = DictNoNone()
        suite_dict["path"] = self.path
        suite_dict["tag"] = self.tag
        # Ref whYeq7
        suite_dict["piggybackhost"] = self.piggybackhost
        # Ref FF3Vph
        suite_dict["robot_params"] = self.robot_params
        # Ref au4uPB
        suite_dict["failed_handling"] = self.failed_handling
        return suite_dict

    @property
    def path(self):
        return self.suite_dict["path"]

    @property
    def tag(self):
        return self.suite_dict.get("tag", None)

    @property
    def piggybackhost(self):
        return self.suite_dict.get("piggybackhost", None)

    @property
    def robot_params(self):
        params = copy.deepcopy(self.suite_dict.get("robot_params", {}))
        # Variables: transform the var 'list of tuples' into a dict.
        variables_dict = {}
        for (k1, v1) in params.items():
            if k1 == "variable":
                for t in v1:
                    variables_dict.update({t[0]: t[1]})
        params.update(self.dict_if_set("variable", variables_dict))
        return params

    @property
    def failed_handling(self):
        """Get failed_handling dict and convert CMK 2.5 tuple format to dict"""
        fh = self.suite_dict.get("failed_handling", {})
        if not fh:
            return {}
        
        # CMK 2.5 returns strategy as tuple: ('incremental', {'rerun_selection': {...}})
        # Convert to dict format: {'name': 'incremental', 'rerun_selection': {...}}
        if "strategy" in fh and isinstance(fh["strategy"], tuple):
            strategy_name, strategy_params = fh["strategy"][0], fh["strategy"][1]
            new_strategy = {"name": strategy_name}
            if strategy_name == "incremental" and strategy_params:
                new_strategy.update(strategy_params)
            fh = dict(fh)  # Make a copy
            fh["strategy"] = new_strategy
        
        return fh

    @property
    def suiteid(self):
        """Create a unique ID from the Robot path (dir/.robot file) and the tag.
        with underscores for everything but letters, numbers and dot."""
        if bool(self.tag):
            tag_suffix = "_%s" % self.tag
        else:
            tag_suffix = ""
        composite = "%s%s" % (self.path, tag_suffix)
        outstr = re.sub(r"[^A-Za-z0-9\.]", "_", composite)

        # make underscores unique
        return re.sub("_+", "_", outstr).lower()

    @staticmethod
    # Return a dict with key:value only if value is set
    def dict_if_set(key, value):
        if bool(value):
            return {key: value}
        else:
            return {}


# This class is common with CMK 1/2
class RMK:
    def __init__(self, conf):
        self.execution_mode = conf["execution_mode"][0]
        mode_conf = conf["execution_mode"][1]
        self.cfg_dict = {
            "global": DictNoNone(),
            "suites": DictNoNone(),
        }
        # handy dict shortcuts
        global_dict = self.cfg_dict["global"]
        suites_dict = self.cfg_dict["suites"]
        global_dict["execution_mode"] = self.execution_mode
        
        # CMK 2.5 may return tuples for CascadingDropdown without parameters
        # Extract first element: ('zlib_codec', None) → 'zlib_codec'
        agent_output_encoding = conf["agent_output_encoding"]
        if isinstance(agent_output_encoding, tuple):
            global_dict["agent_output_encoding"] = agent_output_encoding[0]
        else:
            global_dict["agent_output_encoding"] = agent_output_encoding
        
        # CMK 2.5 may pass "true"/"false" strings instead of booleans - normalize to boolean
        transmit_html_val = conf["transmit_html"]
        if isinstance(transmit_html_val, str):
            global_dict["transmit_html"] = transmit_html_val.lower() == "true"
        else:
            global_dict["transmit_html"] = transmit_html_val
        
        global_dict["log_level"] = conf["log_level"]
        
        # CMK 2.5 may return tuples for CascadingDropdown without parameters
        # Extract first element: (7, None) → 7
        log_rotation = conf["log_rotation"]
        if isinstance(log_rotation, tuple):
            global_dict["log_rotation"] = log_rotation[0]
        else:
            global_dict["log_rotation"] = log_rotation
        
        global_dict["robotdir"] = conf["dirs"].get("robotdir", None)
        global_dict["outputdir"] = conf["dirs"].get("outputdir", None)
        global_dict["logdir"] = conf["dirs"].get("logdir", None)

        if self.execution_mode == "agent_serial":
            global_dict["cache_time"] = mode_conf["cache_time"]
            global_dict["execution_interval"] = mode_conf["execution_interval"]
            self.execution_interval = mode_conf["execution_interval"]
        elif self.execution_mode == "external":
            # For now, we assume that the external mode is meant to execute all
            # suites exactly as configured. Hence, we can use the global cache time.
            global_dict["cache_time"] = mode_conf["cache_time"]

        if len(mode_conf["suites"]) > 0:
            for suite_dict in mode_conf["suites"]:
                suite = RMKSuite(suite_dict)
                if suite.suiteid in self.cfg_dict["suites"]:
                    raise MKGeneralException(
                        "Error in bakery plugin 'robotmk': Suite with ID %s is not unique. Please use tags to solve this problem."
                        % suite.suiteid
                    )

                self.cfg_dict["suites"].update({suite.suiteid: suite.suite2dict})
        pass

    @property
    def global_dict(self):
        return self.cfg_dict["global"]

    @property
    def suites_dict(self):
        return self.cfg_dict["suites"]

    def controller_plugin(self, opsys: OS) -> Plugin:
        return Plugin(
            base_os=opsys,
            source=Path("robotmk.py"),
        )

    def runner_plugin(self, opsys: OS) -> Plugin:
        if self.execution_mode == "external":
            # Runner and Controller have to be deployed as bin
            # $OMD_ROOT/lib/python3/cmk/base/cee/bakery/core_bakelets/bin_files.py
            pass
        elif self.execution_mode == "agent_serial":
            # the runner plugin gets
            return Plugin(
                base_os=opsys,
                source=Path("robotmk-runner.py"),
                timeout=self.execution_interval - 5,
                interval=self.execution_interval,
            )
        else:
            raise MKGeneralException(
                "Error: Execution mode %s is not supported." % self.execution_mode
            )

    def yml(self, opsys: OS, config) -> PluginConfig:
        return PluginConfig(
            base_os=opsys,
            lines=_get_yml_lines(config),
            target=Path("robotmk.yml"),
            include_header=True,
        )

    def bin_files(self, opsys: OS):
        files = []
        if self.execution_mode == "external":
            for file in "robotmk.py robotmk-runner.py".split():
                files.append(
                    SystemBinary(
                        base_os=opsys,
                        source=Path("plugins/%s" % file),
                        target=Path(file),
                    )
                )
        return files


def get_robotmk_files(conf) -> FileGenerator:
    # ALWAYS (!) make a deepcopy of the conf dict. Even if you do not change
    # anything on it, there are strange changes ocurring while building the
    # packages of OS. A deepcopy solves this completely.
    config = RMK(copy.deepcopy(conf))
    
    # Checkmk 2.5+? hack - don't want to spend too much time on making the code compatible with all versions.
    if type(config.global_dict.get("log_rotation")) == str and config.global_dict.get("log_rotation").startswith("value_"):
        # extract the number 
        config.global_dict["log_rotation"] = int(config.global_dict["log_rotation"].split("_")[1])
    for suite in config.suites_dict.values():
        # Remove empty lists from robot_params - iterate over list copy to avoid RuntimeError
        for robot_param in list(suite.get("robot_params", {}).keys()):
            if (isinstance(suite["robot_params"][robot_param], list) and len(suite["robot_params"][robot_param]) == 0) \
                or (isinstance(suite["robot_params"][robot_param], str) and len(suite["robot_params"][robot_param]) == 0):
                suite["robot_params"].pop(robot_param) 
    for base_os in [OS.LINUX, OS.WINDOWS]:
        controller_plugin = config.controller_plugin(base_os)
        runner_plugin = config.runner_plugin(base_os)
        robotmk_yml = config.yml(base_os, config)
        bin_files = config.bin_files(base_os)
        yield controller_plugin
        # in external mode, the runner is only in bin
        if bool(runner_plugin):
            yield runner_plugin
        yield robotmk_yml
        for file in bin_files:
            yield file


def _get_yml_lines(config) -> List[str]:

    header = (
        "# This file is part of Robotmk, a module for the integration of Robot Framework\n"
        + "# test results into Checkmk.\n"
        + "#\n"
        + "# ROBOTMK VERSION: %s\n" % ROBOTMK_VERSION
        + "# For support, training and consulting, please visit:\n"
        + "# https://robotmk.org\n"
    )
    headerlist = header.split("\n")
    # PyYAML is very picky with Dict subclasses; add a representer to dump the data.
    # https://github.com/yaml/pyyaml/issues/142#issuecomment-732556045
    yaml.add_representer(
        DictNoNone,
        lambda dumper, data: dumper.represent_mapping(
            "tag:yaml.org,2002:map", data.items()
        ),
    )
    bodylist = yaml.dump(
        config.cfg_dict, default_flow_style=False, allow_unicode=True, sort_keys=True
    ).split("\n")
    return headerlist + bodylist


register.bakery_plugin(
    name="robotmk",
    files_function=get_robotmk_files,
)
