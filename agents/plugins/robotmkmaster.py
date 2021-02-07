#!/usr/bin/env python3
# (c) 2020 Simon Meggle <simon.meggle@elabit.de>

# This file is part of RobotMK, a module for the integration of Robot
# framework test results into Checkmk.
# https://robotmk.org
# https://github.com/simonmeggle/robotmk
# https://robotframework.org/#tools

# RobotMK is free software;  you can redistribute it and/or modify it
# under the  terms of the  GNU General Public License  as published by
# the Free Software Foundation in version 3.  This file is distributed
# in the hope that it will be useful, but WITHOUT ANY WARRANTY;  with-
# out even the implied warranty of  MERCHANTABILITY  or  FITNESS FOR A
# PARTICULAR PURPOSE. See the  GNU General Public License for more de-
# ails.  You should have  received  a copy of the  GNU  General Public
# License along with GNU Make; see the file  COPYING.  If  not,  write
# to the Free Software Foundation, Inc., 51 Franklin St,  Fifth Floor,
# Boston, MA 02110-1301 USA.


from pathlib import Path
import os
import re
from argparse import ArgumentParser
from datetime import datetime
from time import time
import json 
import logging
from logging.handlers import TimedRotatingFileHandler

# TODO: Write spoolfile also in async mode
# TODO: Overwrite also Defaults with env vars. 


class RMKConfig():
    _PRESERVED_WORDS = [
        'agent_output_encoding',
        'execution_mode',
        'log_rotation',
        'cache_time',
    ]
    # keys that can follow a suite id (to preserve suite ids from splitting)
    _SUITE_SUBKEYS = '''name suite test include exclude critical noncritical
        variable variablefile exitonfailure host'''.split()

    def __init__(self, calling_cls, prefix='ROBOTMK_',
                 preserved_words=_PRESERVED_WORDS,
                 suite_subkeys=_SUITE_SUBKEYS):
        self.calling_cls = calling_cls

        envdict = self.read_env2dictionary(
            prefix=prefix, preserved_words=preserved_words,
            suite_subkeys=suite_subkeys)
        robotmk_yml = Path(self.get_robotmk_var(
            'agent_config_dir')).joinpath(
            self.get_robotmk_var('robotmk_yml'))
        robotmk_dict = self.read_robotmk_yml(robotmk_yml)
        # merge I: combine the os and noarch defaults
        defaults = calling_cls._DEFAULTS
        defaults_dict = {
            'global': mergedeep.merge(defaults[os.name], defaults['noarch'])
        }
        # merge II: YML config overwrites the defaults
        robotmk_dict_merged = mergedeep.merge(robotmk_dict, defaults_dict)
        # merge III: environment vars overwrite the YML config
        self._merged_dict = mergedeep.merge(robotmk_dict_merged, envdict)

    @property
    def global_dict(self):
        return self._merged_dict['global']

    @property
    def suites_dict(self):
        return self._merged_dict['suites']

    @suites_dict.setter
    def suites_dict(self, suites_dict):
        self._merged_dict['suites'] = suites_dict

    @property
    def suites(self):
        return list(self.suites_dict.keys())

    @staticmethod
    def gen_nested_dict(keys, value):
        '''Generates a nested dict from list of keys

        Args:
            keys (list): list of key strings
            value (str/int): the leaf value

        Returns:
            dict: A nested dict with the depth of len(keys) and value=value
        '''
        new_dict = current = {}
        for idx, key in enumerate(keys):
            current[key] = {}
            if key != keys[-1]:
                current = current[key]
            else:
                current[key] = value
        return new_dict

    @classmethod
    def read_env2dictionary(cls, prefix, preserved_words, suite_subkeys):
        '''Creates a nested dict from environment vars starting with a certain
        prefix. Keys are spearated by "_". Preserved words (which already
        contain underscores) are given as a list of preserved words.

        Args:
            prefix (str): Only scan environment variables starting with this
            prefix preserved_words (list): List of words not to split at
                underscores
            suite_subkeys (list): List of words which can occurr after suite id
        Returns:
            dict: A nested dict holding the values from env vars.
        '''
        env_dict = {}
        for varname in os.environ:
            if not varname.startswith(prefix):
                continue
            else:
                varname_strip = varname.replace(prefix, '')
                candidates = []
                for subkey in suite_subkeys:
                    # suite ids have to be treated as preserved words
                    match = re.match(rf'.*SUITES_(.*)_{subkey.upper()}',
                                     varname_strip)
                    if match:
                        candidates.append(match.group(1))
                if len(candidates) > 0:
                    # take only the longest match because suite ids can contain
                    # preserved words (e.g. "SELENIUM_TEST")
                    longest_match = max(candidates, key=len)
                    preserved_words.append(longest_match)
                for pw in preserved_words:
                    pw = pw.upper()
                    if pw in varname_strip:
                        varname_strip = varname_strip.replace(
                            pw, pw.replace('_', '#'))
            list_of_keys = [
                key.replace('#', '_').lower()
                for key in varname_strip.split('_')]
            nested_dict = cls.gen_nested_dict(list_of_keys, os.environ[varname])
            env_dict = mergedeep.merge(env_dict, nested_dict)
        return env_dict

    def get_robotmk_var(self, varname):
        '''Tries to read a ROBOTMK_ var, otherwise returns the OS default value.
        Args:
            varname (str): The setting name
        Returns:
            any: Value of environment var or the OS default.
        '''
        # read from env
        value = self.get_robotmk_env(varname)
        if value is None:
            # read from OS defaults
            return self.get_os_default(varname)

    @staticmethod
    def get_robotmk_env(setting, default=None):
        '''Try to read an environment variable starting with ROBOTMK_ or return default
        Args:
            setting (str): Name of the varname part after the prefix
            default (any, optional): Default value if variable not found.
        Returns:
            any: The value of environment variable ROBOTMK_$setting
        '''
        varname = 'ROBOTMK_' + setting.upper()
        return os.environ.get(varname, default)

    def get_os_default(self, setting):
        '''Read a setting from the DEFAULTS hash. If no OS setting is found, try noarch.
        Args:
            setting (str): Setting name
        Returns:
            str: The setting value
        '''
        value = self.calling_cls._DEFAULTS[os.name].get(setting, None)
        if value is None:
            value = self.calling_cls._DEFAULTS['noarch'].get(setting, None)
            if value is None:
                # TODO: Exception
                pass
        return value

    def read_robotmk_yml(self, file):
        if os.access(file, os.R_OK):
            self.calling_cls.logdebug(f'Reading configuration file {file}')
            try:
                with open(file, 'r') as stream:
                    robotmk_yml_config = yaml.safe_load(stream)
                return robotmk_yml_config
            except yaml.YAMLError as exc:
                self.calling_cls.logerror("Error while parsing YAML file:")
                if hasattr(exc, 'problem_mark'):
                    self.calling_cls.logerror(f'''Parser says: {str(exc.problem_mark)}
                             {str(exc.problem)} {str(exc.context)}''')
                    exit(1)
        else:
            self.calling_cls.logerror(f"ERROR: {file} does not exist. Exiting.")
            exit(1)


# class RMKSuites():
#     # TODO: Suites container needed?
#     def __init__(self, config):
#         self._suites = [
#             RMKSuite(id, config)
#             for id in config.suites ]
#         pass

#     @property
#     def suites(self):
#         return self._suites
        

class RMKSuite():
    def __init__(self, id, config):
        self._start_time = None
        self._end_time = None
        self.id = id
        global_config = config.global_dict
        self.cache_time = global_config['cache_time']
        suite_config = config.suites_dict[id]
        now = int(time())
        # robotframework_mytestsuite_164423445-output.xml
        suite_filename = "robotframework_%s_%s" % (id, str(now))
        # path is the only non-RF key in suite dict. Move this up
        self.path = suite_config.pop('path')
        suite_config.update({
            'outputdir':  global_config['outputdir'],
            'output': f'{suite_filename}_output.xml',
            'log':    f'{suite_filename}_log.html',
            'report': f'{suite_filename}_report.html',
        })
        self._config = {
            'global': global_config,
            'suite': suite_config
        }
        self.spoolfile = RMKSpoolfile(self)

    @property
    def config(self):
        return self._config

    @property
    def start_time(self):
        return self._start_time

    @start_time.setter
    def start_time(self, t):
        #TODO: start time validation
        self._start_time = t

    @property
    def end_time(self):
        return self._end_time

    @end_time.setter
    def end_time(self, t):
        #TODO: end time validation
        self._end_time = t

# {
# 	'suite1_with_vars': {
#         "last_start_time": "2020-11-20 11:45:33", 
#         "last_end_time": "2020-11-20 11:47:29",
#         "last_rc": 0,
#         "last_message": "OK: suite1_with_vars (00:01:46)"
# 	    "xml": "C:\Windows\temp\robotmk_suite1_with_vars_12134242-output.xml",
# 	    "html": "C:\Windows\temp\robotmk_suite1_with_vars_12134242-log.html",
# 	}
# }

class RMKSpoolfile():
    def __init__(self, suite):
        self.suite = suite

    @property
    def mtime(self):
        '''Returns:
            [int]: mtime of the suite's spool file. 0 if not present.'''  
        try:
            mtime = datetime.fromtimestamp(
                os.path.getmtime(str(self.path)))
        except Exception:
            # file not found etc.
            mtime = -1
        return mtime

    @property
    def path(self):
        cache_time = self.suite.config['global']['cache_time']
        filename = f'{str(cache_time)}_robotmk_{self.suite.id}.json'
        return Path(self.suite.config['global']['agent_spool_dir']
                ).joinpath(filename)

    def write(self):
        '''Writes the spoolfile content for a executed suite'''
        pass


class RobotMK():
    _DEFAULTS = {
        'nt': {
            'agent_data_dir': 'C:/ProgramData/checkmk/agent',
            'agent_config_dir': 'C:/ProgramData/checkmk/agent/config',
            'agent_spool_dir': 'C:/ProgramData/checkmk/agent/spool',
            'outputdir': "C:/Windows/temp",
            'logdir': "C:/Windows/temp",
        },
        'posix': {
            'agent_data_dir': '/usr/lib/check_mk_agent',
            'agent_config_dir': '/etc/check_mk',
            'agent_spool_dir': '/var/lib/check_mk_agent/spool',
            'outputdir': "/tmp/robot",
            'logdir': "/var/log/",
        },
        'noarch': {
            'robotmk_yml': 'robotmk.yml',
            'logging': True
        }
    }

    def __init__(self): 
        self.__setup_logging(self.__class__, self.cmdline_args.verbose)
        self.loginfo("foo")
        self._config = RMKConfig(calling_cls=self)
        # self._suites = [ RMKSuite(id, self._config) for id in self._config.suites ]

    @classmethod
    def get_args(cls):
        parser = ArgumentParser()
        parser.add_argument('--run',
                            '-r',
                            dest='suites',
                            const='all',
                            default=None,
                            action='store',
                            nargs='?',
                            type=str,
                            help="""Run all Robot Framework suites as configured in robotmk.yml.
                                    Suite IDs can be given as comma separated list to restrict execution.
                                    Suites are executed serially, one by one.""")
        parser.add_argument('--verbose',
                            '-v',
                            default=False,
                            action='store_true',
                            help="""Print the RobotMK log to console.""")
        cls.cmdline_args = parser.parse_args()

    
    def __setup_logging(self, cls, verbose=False):
        if self._DEFAULTS['noarch']['logging']:
            instance_name = cls.__name__
            logger = logging.getLogger(instance_name)
            logger.setLevel(logging.DEBUG)
    
            # File log
            fh = TimedRotatingFileHandler(
                Path(self._DEFAULTS[os.name]['logdir']).joinpath(f'robotmk.log'),
                when="h", interval=24, backupCount=30)
            file_formatter = logging.Formatter(
                fmt='%(asctime)s %(name)10s %(levelname)8s: %(message)s')
            fh.setFormatter(file_formatter)
            fh.setLevel(logging.DEBUG)
            logger.addHandler(fh)
            self.logger = logger
            # stdout
            if verbose: 
                console = logging.StreamHandler()
                console_formatter = logging.Formatter(
                    fmt='%(asctime)s %(name)10s %(levelname)8s: %(message)s')
                console.setFormatter(console_formatter)
                console.setLevel(logging.DEBUG)
                self.logger.addHandler(console)

    def logdebug(self, text):
        self.logger.debug(text)

    def loginfo(self, text):
        self.logger.info(text)

    def logwarn(self, text):
        self.logger.warn(text)

    def logerror(self, text):
        self.logger.error(text)

    @property
    def config(self):
        return self._config


class RMKplugin(RobotMK):
    def __init__(self, suites2start): 
        super().__init__()
        self._suites2start = [ x.strip() for x in suites2start.split(',') ]
        # magic word "all": start all suites as configured
        if len(self._suites2start) == 1 and self._suites2start[0] == "all":
            # if config.suites empty => execute all => set dummies for dirs within robot dir
            if len(self.config.suites) == 0:
                self.config.suites_dict = {suitedir.name: {} for suitedir in 
                  Path(self.config.global_dict['robotdir']).iterdir()}
            self._suites2start = self.config.suites
            pass

        # start only suites given as arg, warn about the rest 
        else: 
            suites_not_in_cfg = [suite for suite in self._suites2start 
                                 if suite not in self.config.suites]
            if len(suites_not_in_cfg) > 0:
                self.logwarn("Got suites to start which are not in the" +
                             f"YML config: {','.join(suites_not_in_cfg)}")
            self._suites2start = [
                suite for suite in self.config.suites 
                if suite in self._suites2start]

    def run(self):
        '''Execute the suites2start'''        


class RMKctrl(RobotMK):
    def __init__(self):
        super().__init__()
        self.result_spoolfiles = []


    def check_spoolfiles(self):
        '''Check the state of spool files for staleness'''    
        for suite in self.config.suites:
            cache_time = int(suite.config['global']['cache_time'])
            now = int(time())
            self.result_spoolfiles.append("%s;%d;%d;%d" % (
                suite.id,
                cache_time,
                suite.spoolfile.mtime,
                # overdue seconds
                now - suite.spoolfile.mtime - cache_time,
            ))


def test_for_modules():
    try:
        global yaml
        import yaml
        global robot
        import robot
        global mergedeep
        import mergedeep
    except ModuleNotFoundError as e:
        logger.error(f'Could not start because of a missing module: {str(e)}')
        exit(1)



if __name__ == '__main__':
    test_for_modules()
    RobotMK.get_args()
    # Read the configuration from robotmk.yml & environment
    if RobotMK.cmdline_args.suites is None:
        # "Controller" Mode
        #TODO: Mock spoolfiles
        #TODO: Read Spoolfiles, generate Output
        #TODO: Monitor Spoolfile for staleness
        rmk_ctrl = RMKctrl()
        
        rmk_ctrl.check_spoolfiles()
        pass

    else:
        # "Plugin" Mode
        #TODO: old plugin should write metadata spoolfiles to tmpdir
        
        rmk_plugin = RMKplugin(RobotMK.cmdline_args.suites)
        rmk_plugin.run()
        print("Executing Plugin, suites: %s" % robotmk_args.suites)



# <<<robotmk>>>  =  Robot Suite Results
