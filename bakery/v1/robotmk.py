#!/usr/bin/env python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

# (c) 2021 Simon Meggle <simon.meggle@elabit.de>

# This file is part of Robotmk
# https://robotmk.org
# https://github.com/simonmeggle/robotmk

# Robotmk is free software;  you can redistribute it and/or modify it
# under the  terms of the  GNU General Public License  as published by
# the Free Software Foundation in version 3.  This file is distributed
# in the hope that it will be useful, but WITHOUT ANY WARRANTY;  with-
# out even the implied warranty of  MERCHANTABILITY  or  FITNESS FOR A
# PARTICULAR PURPOSE. See the  GNU General Public License for more de-
# ails.  You should have  received  a copy of the  GNU  General Public
# License along with GNU Make; see the file  COPYING.  If  not,  write
# to the Free Software Foundation, Inc., 51 Franklin St,  Fifth Floor,
# Boston, MA 02110-1301 USA.

ROBOTMK_VERSION = 'v1.1.1'

import cmk.utils.paths
import os
import yaml
import re
import copy
from cmk.utils.exceptions import MKGeneralException


DEFAULTS = {
    'windows': {
        'newline': "\r\n",
    },
    'linux': {
        'newline': "\n",
    },
    'posix': {
        'newline': "\n",
    },
    'noarch': {
        'cache_time': 900,
    }
}

def bake_robotmk(opsys, conf, conf_dir, plugins_dir):
    # ALWAYS (!) make a deepcopy of the conf dict. Even if you do not change
    # anything on it, there are strange changes ocurring while building the
    # packages of OS. A deepcopy solves this completely.
    myconf = copy.deepcopy(conf)
    execution_mode = myconf['execution_mode'][0]
    if opsys not in ['windows', 'linux']:
        raise MKGeneralException(
            "Error in bakery plugin 'robotmk': Robotmk is only supported on Windows and Linux."
        )
    config = RMK(myconf, opsys, execution_mode)

    # Robotmk RUNNER plugin
    # executed async, OS-specific
    if execution_mode == "agent_serial":
        if opsys == "windows":
            # async mode in Windows: write configfile in INI-style, will be converted
            # during installation to YML
            with Path(conf_dir, "check_mk.ini.plugins.robotmk-runner.py").open("w") as out:
                out.write(u"    execution robotmk-runner.py = async\r\n")
                out.write(u"    cache_age robotmk-runner.py = %d\r\n" % config.global_dict['execution_interval'])
                # Kill the plugin before the next async execution will start
                out.write(u"    timeout robotmk-runner.py = %d\r\n" % config.global_dict['cache_time'])
                out.write(u"\r\n")
                plugins_dir_async = plugins_dir
        elif opsys == "linux":
            # async mode in Linux: "seconds"-subdir in plugins dir
            plugins_dir_async = Path(
                plugins_dir, "%s" % config.global_dict['execution_interval'])
            plugins_dir_async.mkdir(parents=True, exist_ok=True)
        else:
            raise MKGeneralException(
                "Error in bakery plugin \"%s\": %s\n" %
                ("robotmk", "Robotmk is supported on Windows and Linux only"))

        src = str(
            Path(cmk.utils.paths.local_agents_dir).joinpath('plugins/robotmk-runner.py'))
        dest = str(Path(plugins_dir_async).joinpath('robotmk-runner.py'))

        shutil.copy2(src, dest)
    elif execution_mode == "external":
        # In CMK1 and external mode, the custom package "robotmk" must be deployed. 
        pass

    # II) Robotmk Controller plugin
    # executed sync, regular plugin
    src = str(
        Path(cmk.utils.paths.local_agents_dir).joinpath('plugins/robotmk.py'))
    dest = str(Path(plugins_dir).joinpath('robotmk.py'))
    shutil.copy2(src, dest)

    # III) Generate robotmk.YML config file
    with open(conf_dir + "/robotmk.yml", "w") as robotmk_yml:
        yml_lines = get_yml_lines(config)
        for line in yml_lines: 
            robotmk_yml.write(line + config.os_newline)
    pass

def get_yml_lines(config):
    header = "# This file is part of Robotmk, a module for the integration of Robot\n" +\
        "# framework test results into Checkmk.\n" +\
        "#\n" +\
        "# https://robotmk.org\n" +\
        "# https://github.com/simonmeggle/robotmk\n" +\
        "# https://robotframework.org/\n" +\
        "# ROBOTMK VERSION: %s\n" % ROBOTMK_VERSION
    headerlist = header.split('\n')   
    # PyYAML is very picky with Dict subclasses; add a representer to dump the data. 
    # https://github.com/yaml/pyyaml/issues/142#issuecomment-732556045
    yaml.add_representer(
        DictNoNone, 
        lambda dumper, data: dumper.represent_mapping('tag:yaml.org,2002:map', data.items())
        )
    # Unicode representer, see https://stackoverflow.com/a/62207530
    yaml.add_representer(
        unicode, 
        lambda dumper, data: dumper.represent_scalar(u'tag:yaml.org,2002:str', data)
        )

    bodylist = yaml.dump(
        config.cfg_dict,
        default_flow_style=False,
        allow_unicode=True,
        encoding='utf-8',
        sort_keys=True).split('\n')         
    return headerlist + bodylist


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
class RMKSuite():
    def __init__(self, suite_tuple):
        self.suite_tuple = suite_tuple      

    @property
    def suite2dict(self): 
        suite_dict = DictNoNone()
        suite_dict['path']= self.path
        suite_dict['tag']= self.tag
        # Ref whYeq7
        suite_dict['piggybackhost']= self.piggybackhost
        # Ref FF3Vph
        suite_dict['robot_params'] = self.robot_params
        # Ref au4uPB
        suite_dict['failed_handling'] = self.failed_handling
        return suite_dict

    # Ref a01uK3
    @property
    def path(self):
        return self.suite_tuple[0]

    # Ref yJE5bu
    @property
    def tag(self):
        return self.suite_tuple[1].get('tag', None)

    # Ref whYeq7
    @property
    def piggybackhost(self):
        return self.suite_tuple[2].get('piggybackhost', None)

    # Ref FF3Vph
    @property
    def robot_params(self):
        params = copy.deepcopy(self.suite_tuple[3].get('robot_params', {}))
        # Variables: transform the var 'list of tuples' into a dict.
        variables_dict = {}
        for (k1, v1) in params.items():
            if k1 == 'variable':
                for t in v1:
                    variables_dict.update({t[0]: t[1]})
        params.update(self.dict_if_set('variable', variables_dict))    
        return params

    # Ref au4uPB
    @property 
    def failed_handling(self):
        failed_handling = copy.deepcopy(self.suite_tuple[4].get('failed_handling', {}))
        ret = {}
        if failed_handling:
            ret.update({'max_executions': failed_handling[0]})
            ret.update(self.dict_if_set('rerun_selection', failed_handling[1]))
        return ret

    @property
    def suiteid(self):
        '''Create a unique ID from the Robot path (dir/.robot file) and the tag. 
        with underscores for everything but letters, numbers and dot.'''
        if bool(self.tag):
            tag_suffix = "_%s" % self.tag
        else:
            tag_suffix = ""
        composite = "%s%s" % (self.path, tag_suffix)
        outstr = re.sub('[^A-Za-z0-9\.]', '_', composite)
        # make underscores unique
        return re.sub('_+', '_', outstr).lower()

    @staticmethod
    # Return a dict with key:value only if value is set
    def dict_if_set(key, value):
        if bool(value):
            return {key: value}
        else:
            return {}

# This class is common with CMK 1/2

class RMK():
    def __init__(self, conf, opsys, execution_mode):
        self.os_newline = get_os_default('newline', opsys)

        self.execution_mode = conf['execution_mode'][0]
        mode_conf = conf['execution_mode'][1]        
        self.cfg_dict = {
            'global': DictNoNone(),
            'suites': DictNoNone(),
        }
        # handy dict shortcuts
        global_dict = self.cfg_dict['global']
        suites_dict = self.cfg_dict['suites']
        global_dict['execution_mode'] =  self.execution_mode
        global_dict['agent_output_encoding'] =  conf['agent_output_encoding']
        global_dict['transmit_html'] =  conf['transmit_html']
        global_dict['logging'] =  conf['logging']
        global_dict['log_rotation'] =  conf['log_rotation']
        # WATO makes robotdir a nested dict with duplicate key. Form follows function :-/
        global_dict['robotdir'] =  conf.get('robotdir', {}).get('robotdir', None)

        if self.execution_mode == 'agent_serial':
            global_dict['cache_time'] = mode_conf[1]
            global_dict['execution_interval'] = mode_conf[2]
            self.execution_interval = mode_conf[2]
        elif self.execution_mode == 'external':
            # For now, we assume that the external mode is meant to execute all
            # suites exactly as configured. Hence, we can use the global cache time.
            global_dict['cache_time'] = mode_conf[1]  

        if 'suites' in mode_conf[0]:
            # each suite suite_tuple:
            # 0) path, Ref a01uK3
            # 1) tag, Ref yJE5bu
            # 2) piggybackhost, Ref whYeq7
            # 3) robot_params{}, Ref FF3Vph
            # 4) failed_handling, Ref au4uPB            
            for suite_tuple in mode_conf[0]['suites']:
                suite = RMKSuite(suite_tuple)
                if suite.suiteid in self.cfg_dict['suites']:
                    raise MKGeneralException(
                        "Error in bakery plugin 'robotmk': Suite with ID %s is not unique. Please use tags to solve this problem." % suite.suiteid 
                    )      

                self.cfg_dict['suites'].update({
                    suite.suiteid: suite.suite2dict})        

        pass

    @property
    def global_dict(self):
        return self.cfg_dict['global']

    @property
    def suites_dict(self):
        return self.cfg_dict['suites']

def get_os_default(setting, opsys):
    '''Read a setting from the DEFAULTS hash. If no OS setting is found, try noarch.
    Args:
        setting (str): Setting name
    Returns:
        str: The setting value
    '''
    value = DEFAULTS[opsys].get(setting, None)
    if value is None:
        value = DEFAULTS['noarch'].get(setting, None)
        if value is None:
            raise MKGeneralException(
                "Error in bakery plugin 'robotmk': Cannot find setting '%s' for OS %s." % (setting, opsys))
    return value




def make_suiteid(robotpath, tag):
    '''Create a unique ID from the Robot path (dir/.robot file) and the tag. 
    with underscores for everything but letters, numbers and dot.'''
    if bool(tag):
        tag_suffix = "_%s" % tag
    else:
        tag_suffix = ""
    composite = "%s%s" % (robotpath, tag_suffix)
    outstr = re.sub('[^A-Za-z0-9\.]', '_', composite)
    # make underscores unique
    return re.sub('_+', '_', outstr).lower()


bakery_info["robotmk"] = {
    "bake_function": bake_robotmk,
    "os": ["linux", "windows"],
}


