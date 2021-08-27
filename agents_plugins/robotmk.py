#!/usr/bin/env python3
# (c) 2021 Simon Meggle <simon.meggle@elabit.de>

# This file is part of Robotmk, a module for the integration of Robot
# framework test results into Checkmk.
# https://robotmk.org
# https://github.com/simonmeggle/robotmk
# https://robotframework.org/#tools

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

# This plugin requires Python > 3.7 and some modules:
# pip3 install robotframework pyyaml mergedeep python-dateutil

# redirect stdout while testing: https://www.devdungeon.com/content/using-stdin-stdout-and-stderr-python

from pathlib import Path
from collections import defaultdict
import os
import sys
import re
from argparse import ArgumentParser, RawTextHelpFormatter
from datetime import datetime, timezone, timedelta
from time import time
import json
import inspect
import base64
import zlib
import logging
from logging.handlers import TimedRotatingFileHandler
from textwrap import dedent
import subprocess
import platform
import xml.etree.ElementTree as ET
from enum import Enum
from abc import ABC, abstractmethod 
import glob
import contextlib

from robot.rebot import rebot


local_tz = datetime.utcnow().astimezone().tzinfo
ROBOTMK_VERSION = 'v1.2-beta'

class RMKConfig():
    _PRESERVED_WORDS = [
        'agent_output_encoding',
        'execution_mode',
        'log_rotation',
        'cache_time',
        'execution_interval',
    ]
    # keys that can follow a suite id (to preserve suite ids from splitting)
    _SUITE_SUBKEYS = '''name suite test include exclude critical noncritical
        variable variablefile exitonfailure host'''.split()

    def __init__(self, calling_cls):
        self.calling_cls = calling_cls
        # merge I: combine the os and noarch defaults
        defaults_dict = self.__merge_defaults()
        # merge II: YML config overwrites the defaults
        robotmk_dict = self.read_robotmk_yml()
        robotmk_dict_merged_default = mergedeep.merge(
            robotmk_dict, defaults_dict)
        # merge III: environment vars overwrite the YML config
        envdict = self.read_env2dictionary()
        robotmk_dict_merged_env = mergedeep.merge(
            robotmk_dict_merged_default, envdict)

        self.cfg_dict = robotmk_dict_merged_env
        # Determine the default robotdir path, if no custom one was given
        if not 'robotdir' in self.cfg_dict['global']: 
            self.cfg_dict['global'].update({
                'robotdir' : Path(self.calling_cls._DEFAULTS[os.name]['agent_data_dir']).joinpath('robot')
            })
        # now that YML and ENV are read, see if there is any suite defined.
        # If not, the fallback is generate suite dict entries for every dir
        # in robotdir.
        if len(self.suites_dict) == 0:
            self.suites_dict = self.__suites_from_robotdirs()


    def __merge_defaults(self):
        defaults = self.calling_cls._DEFAULTS
        merged_defaults = {
            'global': mergedeep.merge(defaults[os.name], defaults['noarch'])
        }
        return merged_defaults

    def __suites_from_robotdirs(self):
        self.calling_cls.loginfo(
            'No suites defined in YML and ENV; seeking for dirs in %s/...' %
            self.global_dict['robotdir'])
        # Collect all .robot files and all directories (ecept hidden ones like .vscode)
        suites_dict = {
            suitedir.name: {
                'path': suitedir.name,
                'tag': '',
            } for suitedir in
            [ x for x in Path(self.global_dict['robotdir']).iterdir() if (x.is_dir() or x.name.endswith('.robot')) and not x.name.startswith('.') ]
            }
        return suites_dict

    @property
    def lsuites(self):
        return self.cfg_dict['suites'].keys()

    def suite_objs(self, logger):
        return [RMKSuite(id, self, logger) for id in self.cfg_dict['suites']]

    @property
    def global_dict(self):
        return self.cfg_dict['global']

    @property
    def suites_dict(self):
        return self.cfg_dict['suites']

    @suites_dict.setter
    def suites_dict(self, suites_dict):
        self.cfg_dict['suites'] = suites_dict

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

    def read_env2dictionary(self, prefix='ROBOTMK_',
                            preserved_words=_PRESERVED_WORDS,
                            suite_subkeys=_SUITE_SUBKEYS):
        '''Creates a nested dict from environment vars starting with a certain
        prefix. Keys are spearated by "_". Preserved words (which already
        contain underscores) are given as a list of preserved words.

        Args:
            prefix (str): Only scan environment variables starting with this
                prefix
            preserved_words (list): List of words not to split at
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
                self.calling_cls.logdebug(f'ENV: Found variable {varname}')
                varname_strip = varname.replace(prefix, '')
                candidates = []
                for subkey in suite_subkeys:
                    # suite ids have to be treated as preserved words
                    match = re.match(rf'.*suites_(.*)_{subkey}',
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
                key.replace('#', '_')
                for key in varname_strip.split('_')]
            # TODO: Suite names with underscores are not parsed correctly!
            nested_dict = self.gen_nested_dict(
                list_of_keys, os.environ[varname])
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
                # TODO: Catch the exception!
                pass
        return value

    def read_robotmk_yml(self):
        robotmk_yml = Path(self.get_robotmk_var(
            'agent_config_dir')).joinpath(
            self.get_robotmk_var('robotmk_yml'))
        if os.access(robotmk_yml, os.R_OK):
            self.calling_cls.logdebug(
                f'Reading configuration file {robotmk_yml}')
            # TEST: Reading a valid robotmk.yml
            try:
                with open(robotmk_yml, 'r', encoding='utf-8') as stream:
                    robotmk_yml_config = yaml.safe_load(stream)
                return robotmk_yml_config
            except yaml.YAMLError as exc:
                self.calling_cls.logerror("Error while parsing YAML file:")
                if hasattr(exc, 'problem_mark'):
                    self.calling_cls.logerror(f'''Parser says: {str(exc.problem_mark)}
                             {str(exc.problem)} {str(exc.context)}''')
                    exit(1)
        else:
            # TEST: Valid config 100% from environment (-> Docker!)
            self.calling_cls.loginfo("No control file %s found. ")
            return {}


class RMKState():
    '''State class which is the superclass for runner and suite.
    Both share the fact that
    - they store some common data like runtime, cache time etc.
    - they need to store those data in the state file
    - some data in the state file must be updated in real-time'''

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)  # forwards all unused arguments
        self._state = {}

    def read_state_from_file(self):
        try:
            with open(str(self.statefile_path), "r", encoding='utf-8') as statefile:
                data = json.load(statefile)
            # statefile always contains ISO datetimes, convert them back to datetime
            data = {k: (parser.isoparse(v) if type(v) is datetime else v)
                    for (k, v) in data.items()}
        except Exception as e:
            # TODO: Not optimal. Logging is only inherited from RoboMK to Ctrl and Runner.
            # self.logwarn("Statefile not found - %s (%s)" % (self.statefile_path, str(e)))
            data = {}
            # TODO: Test
            # data = {
            #     'id': self.suite.id,
            #     'error': "Statefile of suite '%s' not found - %s (perhaps the suite did never run)" % (self.suite.id, str(e))
            # }

        # self.data['result_age'] = self.age.seconds
        # self.data['result_overdue'] = self.overdue
        # self.data['result_is_stale'] = self.is_stale()
        return data

    def write_state_to_file(self, data=None):
        """Writes the given data structure into the statefile.
        Datetime objects are converted to ISO strings."""
        if data is None:
            data = self._state        
        data = {k: (v.isoformat() if type(v) is datetime else v)
                for (k, v) in data.items()}
        try:
            with open(self.statefile_path, 'w', encoding='utf-8') as outfile:
                json.dump(data, outfile, indent=2, sort_keys=False)
        except IOError as e:
            # Error gets logged, will come to light by staleness check
            pass
            # TODO: Not optimal. Logging is only inherited from RoboMK to Ctrl and Runner.
            # self.logerror("Cannot write statefile %s" % (
            #     self.statefile_path, str(e)))

    def state_isoformat(self):
        data = {k: v.isoformat() for (k, v) in self._state.items()}

    @property
    def is_running(self):
        '''Checks if the Runner has not ended yet'''
        if self._state['start_time'] > self._state['end_time']:
            return True

    @property
    def is_due(self):
        '''Checks if the runner should run according to the exec. interval'''
        pass
        # if self.now > last_start_time + global_execution_interval:

    @property
    def statefile_path(self):
        # The controller reads the runner's statefile, but does not have an ID.
        # Hence, we fallback to runner, if not set.
        id = getattr(self, 'id', 'runner')
        filename = f'robotmk_{id}.json'
        return Path(self.config.global_dict['outputdir']).joinpath(filename)
        # return Path(self.global_dict['outputdir']).joinpath(filename)

    def write_statevars(self, kvpair):
        if not type(kvpair) is list:
            kvpair = [kvpair]
        self.set_statevars(kvpair)
        data = self.read_state_from_file()
        for item in kvpair:
            data.update({item[0]: item[1]})
        self.write_state_to_file(data)

    def set_statevars(self, kvpair):
        if not type(kvpair) is list:
            kvpair = [kvpair]
        for item in kvpair:
            self._state[item[0]] = item[1]

    def get_statevar(self, name):
        return self._state.get(name, None)

    # def update_file(fn):
    #     # always save the current state to file
    #     def inner(*args, **kwargs):
    #         if not args[0] is None:
    #             print("Writing this to file %s " % "foo")
    #         fn()
    #     return inner

    def get_now_as_dt(self):
        return datetime.now(local_tz)

    def get_now_as_epoch(self):
        return int(self.get_now_as_dt().timestamp())


class RMKSuite(RMKState):
    logmark = '~'

    def __init__(self, id, config, logger):
        self.id = id
        self.config = config
        self.logger = logger
        self._timestamp = self.get_now_as_epoch()
        super().__init__()

        self.set_statevars([
            ('id', id),
            ('cache_time', self.get_suite_or_global('cache_time')),
            ('execution_interval', self.get_suite_or_global('execution_interval')),
            ('path', self.suite_dict['path']),
            ('tag', self.suite_dict.get('tag', None)),
        ])

        self.suite_dict.setdefault('robot_params', {})    
        self.suite_dict['robot_params'].update({
            'outputdir':  self.global_dict['outputdir'],
            'console':  'NONE',
            'report':   'NONE'
        })
        # Ref: TgWQvr
        if self.source == "local":
            self._source_strategy = SourceStrategyFS(path=self.path)
        elif self.source == "git":
            self._source_strategy == SourceStrategyGit(path=self.path)
        elif self.source == "robocorp": 
            self._source_strategy == SourceStrategyRobocorp(path=self.path)
        else: 
            # TODO: catch this
            pass

        if self.python == "os":
            self._env_strategy = EnvStrategyOS(self)
        elif self.python == "rcc":
            self._env_strategy = EnvStrategyRCC(self)
        else: 
            # TODO: catch this
            pass

    def clear_statevars(self):
        data = {k: None for k in 'start_time end_time runtime rc xml htmllog'.split()}
        self._state.update(data)

    def __str__(self):
        return self.id

    def output_filename(self, timestamp, attempt=None):
        """Create output file name. If attempt is given, it gets appended to the file name."""
        if attempt is None: 
            suite_filename = "robotframework_%s_%s" % (self.id, timestamp)
        else:
            suite_filename = "robotframework_%s_%s_attempt-%d" % (self.id, timestamp, attempt)
        return suite_filename

    def update_output_filenames(self, attempt=None):
        """Parametrize the output files"""
        output_prefix = self.output_filename(str(self.timestamp), attempt)
        self.suite_dict['robot_params'].update({
            'output': "%s_output.xml" % output_prefix,
            'log': "%s_log.html" % output_prefix
            })            


    def clear_filenames(self):
        '''Reset the log file names if Robot Framework exited with RC > 250
        The files presumed to exist do not in this case.
        '''
        self.output = None        
        self.log = None


    def run_strategy(self):
        # Ref: TgWQvr
        # start the suite either with OS Python/RCC/Docker
        rc = self._env_strategy.run()
        return rc

    def get_suite_or_global(self, name, default=None):
        try:
            return self.suite_dict[name]
        except:
            try:
                return self.global_dict[name]
            except:
                return default

    @property
    def path(self):
        return Path(self.global_dict['robotdir']
                    # ).joinpath(self.robotpath)
                    ).joinpath(self.suite_dict['path'])

    @property
    def outputdir(self):
        return self.suite_dict['robot_params']['outputdir']
    @outputdir.setter
    def outputdir(self, directory):
        self.suite_dict['robot_params']['outputdir'] = directory

    @property
    def output(self):
        return self.suite_dict['robot_params']['output']
    @output.setter
    def output(self, file):
        self.suite_dict['robot_params']['output'] = file

    @property
    def log(self):
        return self.suite_dict['robot_params']['log']
    @log.setter
    def log(self, file):
        self.suite_dict['robot_params']['log'] = file

    @property
    def runtime(self):
        return (self._state['end_time'] - self._state['start_time']).total_seconds()

    @property
    def python(self): 
        """Defines which Python interpreter to use (OS/RCC)"""
        return self.suite_dict.get('python', 'os')

    @property
    def source(self): 
        return self.suite_dict.get('source', 'local')        

    @property
    def max_executions(self):
        return self.suite_dict.get('failed_handling',{}).get('max_executions',1)

    @property
    def rerun_selection(self):
        return self.suite_dict.get('failed_handling', {}).get('rerun_selection',[])

    @property
    def suite_dict(self):
        return self.config.cfg_dict['suites'][self.id]

    @property
    def global_dict(self):
        return self.config.cfg_dict['global']


    # Suite timestamp for filenames
    @property
    def timestamp(self):
        return self._timestamp
    @timestamp.setter
    def timestamp(self, t):
        self._timestamp = t

# Ref: TgWQvr
class EnvStrategy():
    """Strategy interface which Python environment to use"""
    def __init__(self):
        pass
    @abstractmethod
    def run(self, suite: RMKSuite):
        pass    

class EnvStrategyOS(EnvStrategy):
    """Use the System Python environment"""
    def __init__(self, suite: RMKSuite):
        self._suite = suite
        super().__init__()

    def __str__(self): 
        return("OS Python")


    def prepare_rf_args(self):
        # Format the variables to meet the Robot API requirement
        # --variable name:value --variable name2:value2
        # => ['name:value', 'name2:value2'] (list of dicts to list of k:v)
        variables = self._suite.suite_dict.get('robot_params').get('variable')
        if variables and type(variables) is not list:
            variables = list(
                map(
                    lambda x: f'{x[0]}:{x[1]}',
                    variables.items()
                ))
            self._suite.suite_dict['robot_params']['variable'] = variables
        pass

    def run(self):
        """Runs the Robot suite with the OS Python and RF API"""
        self.prepare_rf_args()        
        rc = robot.run(
            self._suite.path,
            **self._suite.suite_dict.get('robot_params'))
        return rc    

class EnvStrategyRCC(EnvStrategy):
    """Use rcc to create a dedicated environment for the test"""
    def __init__(self, suite: RMKSuite):
        self._suite = suite
    
    def __str__(self): 
        return("RCC Env Python")

    def run(self, suite: RMKSuite) -> int:
        pass    


class SourceStrategy():
    """Strategy interface where to get the test source code from"""
    def __init__(self, path):
        self.path = path
        pass
    
class SourceStrategyFS(SourceStrategy):
    """Read the test source from local filesystem"""
    def __init__(self, path):
        super().__init__(path)
        pass

class SourceStrategyGit(SourceStrategy):
    """Clone the test source code from git"""
    def __init__(self, path):
        super().__init__(path)
        pass

class SourceStrategyRobocorp(SourceStrategy):
    """Load a Robocorp Robot"""
    def __init__(self, path):
        super().__init__(path)
        pass

    
    
    



class RMKPlugin():
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
        self.__setup_logging(
            calling_cls=self, verbose=self.cmdline_args.verbose)
        # self.loginfo("="*20 + " %s " % str(self) + "="*20)
        # self.loginfo(self.logmark * 20 + " %s " % str(self) + self.logmark*20)
        self.loginfo(self.logmark * 20)
        self.config = RMKConfig(calling_cls=self)
        self.execution_mode = self.config.global_dict['execution_mode']

    @classmethod
    def get_args(cls):
        parser = ArgumentParser(
            formatter_class=RawTextHelpFormatter,
            epilog=dedent("""\
                # Operation modes
                Without any arguments, Robotmk works in 'controller mode'. It determines the suites
                which are defined in robotmk.yml to run on this machine. If there are no suites de-
                fined, the suite names are taken from the directory names within the robot suites
                directory.
                If called in 'runner mode', robotmk executes Robot Framework suites. With "--run",
                the default is "all" = run all suites defined (either by YML or by directory
                inspection). If suites are specified as option to "--run", only those are run.

                # Configuration by environment variables
                Any setting can also be given by environment variables.
                Example:

                cat robotmk.yml
                global:
                    robotdir: /another/path/for/suites
                suites:
                    test_one:
                        variable:
                            language: german
                            env: prod

                This can be set equivalentely with environment variables:

                ROBOTMK_global_robotdir="/another/path/for/suites"
                ROBOTMK_suites_test_one_variable_language="german"
                ROBOTMK_suites_test_one_variable_env="prod"

                The rules are:
                  * variables must start with 'ROBOTMK_'
                  * case matters
                  * separate dict keys with underscores
                  * suite names with underscores (ex. test_one) are detected by
                    its surrounding protected keys.
                """))
        # parser.add_argument(
        #     '--run',
        #     '-r',
        #     dest='suites',
        #     const='all',
        #     default=None,
        #     action='store',
        #     nargs='?',
        #     type=str,
        #     help="""runner mode. Runs all Robot Framework suites as configured in robotmk.yml.
        #             Suite IDs can be given as comma separated list to restrict execution.
        #             Suites are executed serially, one by one.""")
        parser.add_argument('--verbose',
                            '-v',
                            default=False,
                            action='store_true',
                            help="""Print the Robotmk log to console.""")
        cls.cmdline_args = parser.parse_args()

    def __setup_logging(self, calling_cls, verbose=False):
        if self._DEFAULTS['noarch']['logging']:
            instance_name = calling_cls.__class__.__name__
            logger = logging.getLogger(instance_name)
            logger.setLevel(logging.DEBUG)

            # File log
            fh = TimedRotatingFileHandler(
                Path(self._DEFAULTS[os.name]['logdir']
                     ).joinpath('robotmk_%s.log' % repr(calling_cls)),
                when="midnight", backupCount=30)
            file_formatter = logging.Formatter(
                fmt='%(asctime)s %(name)10s [%(process)5d] %(levelname)7s: %(message)s')
            fh.setFormatter(file_formatter)
            fh.setLevel(logging.DEBUG)
            logger.addHandler(fh)
            self.logger = logger
            # stdout
            if verbose:
                console = logging.StreamHandler()
                console_formatter = logging.Formatter(
                    fmt='%(asctime)s %(name)10s [%(process)5d] %(levelname)7s: %(message)s')
                console.setFormatter(console_formatter)
                console.setLevel(logging.DEBUG)
                self.logger.addHandler(console)

    def asinstance(f):
        '''Ensures that a function only gets called by instances
        Args:
            logf ([function]): function
        '''
        def wrapper(*args):
            if not inspect.isclass(args[0]):
                f(*args)
        return wrapper

    @asinstance
    def logdebug(self, text):
        self.logger.debug(text)

    @asinstance
    def loginfo(self, text):
        self.logger.info(text)

    @asinstance
    def logwarn(self, text):
        self.logger.warning(text)

    @asinstance
    def logerror(self, text):
        self.logger.error(text)


class RMKrunner(RMKState, RMKPlugin):
    logmark = '#'

    def __init__(self):
        self.id = 'runner'
        super().__init__()
        self.set_statevars([
            ('id', 'runner'),
            ('execution_mode', self.global_dict['execution_mode']),
        ])
        pass

    def __str__(self):
        return 'Robotmk Runner'

    def __repr__(self):
        return 'runner'

    def update_suites2start(self, suites_cmdline):
        '''Updates suites_dict so that it reflects the suites given comma-
        separated on the commandline.
        * '--run' / '--run all': run all suites in cfg; if no suites in config,
                                 run all suites in robotdir
        * '--run suite1,suite3': only run specific suites
        * (no arg)             : (controller mode, do not run any suite)
        Args:
            suites_cmdline (list): comma separated list of suite names
        '''
        suites_cmdline = [x.strip() for x in suites_cmdline.split(',')]
        # to fake an invalid suitename as argument...
        # suites_cmdline = ['foo']
        if (len(suites_cmdline) == 1 and suites_cmdline[0] == "all"):
            # there are no specific suites to run, run all
            self.selective_run = False
            self.loginfo(
                "No suite arguments given to '--run'; will execute all as configured.")
        else:
            self.loginfo(
                "'--run' has suite arguments; merging with list of suites...")
            # There are specific suite arguments
            self.selective_run = True
            # What's configured
            configured_suites = self.config.suites_dict.keys()
            # Suites given as arg do not have a cfg entry:
            suites_inarg_notincfg = [suite for suite in suites_cmdline
                                     if suite not in configured_suites]
            if len(suites_inarg_notincfg) > 0:
                self.logdebug("(+) Adding suites: " +
                              f"'{','.join(suites_inarg_notincfg)}' " +
                              "(not in cfg, but in arguments; assuming this to be a directory name; will try to start this with defaults.)")
                suites_inarg_notincfg_dict = {
                    suiteid: {
                        'path': suiteid
                    } for suiteid in suites_inarg_notincfg}
                self.config.suites_dict.update(suites_inarg_notincfg_dict)

            # Remove suites from cfg which are not given as argument
            keep = {}
            for suiteid, suitedict in self.config.suites_dict.items():
                if suiteid not in suites_cmdline:
                    self.logdebug(
                        f"(-) Skipping suite '{suiteid}'' (in cfg, NOT in arguments)")
                    # del(self.config.suites_dict[suiteid])
                else:
                    self.logdebug(
                        f"( ) Keeping suite '{suiteid}' (in cfg and in arguments)")
                    keep.update({
                        suiteid: self.config.suites_dict[suiteid]
                    })
            self.config.suites_dict = keep
            # self.loginfo("Updated suite list: %s" % ', '.join(keep.keys()))
            pass

    def clear_statevars(self):
        data = {k: None for k in 'start_time end_time runtime runtime_suites runtime_robotmk suites suites_fatal'.split()}
        self._state.update(data)

    def update_runner_statevars(self):
        '''A non-selective (=complete) run is whenever the runner gets started
        with no suite args. That is when:
        - serial mode (controller itself starts runner with no suite args)
        - external mode (a scheduled task starts the runner with no suite args)
        A selective, non-complete run is
        - parallel mode (controller starts one runner per suite)
        - external mode (a scheduled task starts the runner with suite args)'''
        runtime_total = (
            self._state['end_time'] - self._state['start_time']).total_seconds()
        runtime_suites = sum([suite.runtime for suite in self.suites])
        runtime_robotmk = runtime_total - runtime_suites
    
        self.set_statevars([
            ('runtime_total', runtime_total),
            ('runtime_suites', runtime_suites),
            ('runtime_robotmk', runtime_robotmk),
            # ('suites', suites),
            ('selective_run', self.selective_run),
        ])
        if self.execution_mode == 'agent_serial':
            self.set_statevars([('cache_time', self.config.global_dict['cache_time']), (
                'execution_interval', self.config.global_dict['execution_interval'])])
        # elif self.execution_mode == 'agent_parallel':
        #     self.set_statevars([('cache_time', self.config.suite_dict['cache_time']), (
        #         'execution_interval', self.config.suite_dict['execution_interval'])])
        elif self.execution_mode == 'external':
            if self.selective_run:
                self.set_statevars(
                    ('cache_time', self.config.suite_dict['cache_time']))
            else:
                self.set_statevars(
                    ('cache_time', self.config.global_dict['cache_time']))
        else:
            # Better never get here...
            pass

    @property
    def global_dict(self):
        return self.config.cfg_dict['global']

    @property
    def suites_dict(self):
        return self.config.cfg_dict['suites']

    def run_suites(self, suites_cmdline):
        """Executes all suites of robotmk.yml/robotdir"""
        self.update_suites2start(suites_cmdline)
        self.suites = self.config.suite_objs(self.logger)
        self.loginfo(
            ' => Suites to start: %s' % ', '.join([s.id for s in self.suites]))
        self.write_statevars(('start_time', self.get_now_as_dt()))
        for suite in self.suites:
            id = suite.id
            self.loginfo(
                f"{4*RMKSuite.logmark} Suite ID: {id} {4*RMKSuite.logmark}")
            if not os.path.exists(suite.path):
                error = "Suite path %s does not exist. " % suite.path
                self.logerror(error)
                # The statefile will contain iD and error text of this failed
                # suite run. But the controller will only "find" this statefile
                # if he know about it -> if there is a valid entry in the config.
                suite.error = error
                # continue
            self.logdebug(f'Strategy: ' + str(suite._env_strategy) )
            
            rc = self.run_suite(suite)
            

            if rc > 250:
                self.logerror(
                    'RC > 250 = Robot exited with fatal error. There are no logs written.')
                self.logerror(
                    'Please run the robot command manually to debug.')
                suite.clear_filenames()
            self.loginfo(f'Writing suite statefile {suite.statefile_path}')
            suite.write_state_to_file()
        self.set_statevars(('end_time', self.get_now_as_dt()))
        self.update_runner_statevars()
        self.write_state_to_file()


    def merge_results(self, suite):
        # output files without attempt suffix
        suite.update_output_filenames()
        outputfiles = self.glob_suite_outputfiles(suite)
        self.logdebug("Merging the results of the following result files into %s: " % suite.output)
        filenames = [Path(f).name for f in outputfiles]
        for f in filenames: 
            self.logdebug(" - %s" % f)
        # rebot wants to print out the generated file names on stdout; write to devnull
        devnull = open(os.devnull, 'w')                    
        rebot(
            *outputfiles, 
            outputdir=suite.outputdir, 
            output=suite.output,
            log=suite.log,
            report=None,
            merge=True,
            stdout=devnull
            )        

    def run_suite(self, suite):
        """Execute a single suite, including retries"""
        suite.write_statevars([
            ('id', suite.id),
            ('start_time', suite.get_now_as_dt()),
            ('cache_time', suite.get_suite_or_global('cache_time'))
        ])
        max_exec = suite.max_executions
        for attempt in range(1, max_exec+1):
            if max_exec > 1: 
                self.loginfo(f" > Starting attempt {attempt}/{max_exec}...")
            else:
                self.loginfo(f" > Starting suite...")
            # output files with attempt suffix
            suite.update_output_filenames(attempt)
            # The execution
            rc = suite.run_strategy()
            self.loginfo(f" < RC: {rc}")

            if rc == 0:
                if attempt == 1: 
                    # Suite passed on the first try; exit the loop
                    break
                else:
                    # Suite passed on a retry => MERGE
                    self.merge_results(suite)
                    break
            else: 
                if max_exec == 1:
                    # Suite FAILED on the first and only try; exit the loop
                    break
                else: 
                    # Suite FAILED and...
                    if attempt < max_exec:
                        # ...chance for next try!
                        # save the current output XML and use it for the rerun
                        failed_xml = Path(suite.outputdir).joinpath(suite.output)                        
                        suite.suite_dict['robot_params'].update({'rerunfailed': str(failed_xml)})                    
                        # Attempt 2ff can be filtered, add the parameters to the Robot cmdline
                        suite.suite_dict['robot_params'].update(suite.rerun_selection)
                        self.loginfo(f"Re-testing the failed ones in {failed_xml}")
                    else: 
                        # ...GAME OVER! => MERGE
                        self.loginfo("Even the last attempt was unsuccessful!")
                        self.merge_results(suite)
        
        suite.set_statevars([
            ('htmllog', str(Path(suite.outputdir).joinpath(suite.log))),
            ('xml', str(Path(suite.outputdir).joinpath(suite.output))),
            ('end_time', self.get_now_as_dt()),
            ('attempts', attempt),
            ('max_executions', max_exec), 
            ('rc', rc)])    
        self.loginfo(
            f'Final suite RC: {rc}')        
        return rc

    def glob_suite_outputfiles(self, suite):
        """Returns a list of XML output files of all execution attempts"""
        output_filename = suite.output_filename(suite.timestamp)
        outputfiles = [file for file in glob.glob(str(Path(suite.outputdir).joinpath("%s_attempt*_output.xml" % output_filename)))]
        return outputfiles

class RMKCtrl(RMKState, RMKPlugin):
    header = '<<<robotmk>>>'
    logmark = '='

    def __init__(self):
        super().__init__()
        self.cleanup_logs()

    def __str__(self):
        return 'Robotmk Controller'

    def __repr__(self):
        return 'controller'

    def os_popen(self, cmd):
        # FIXME: blocking Agent?

        if platform.system() == 'Linux':
            self.loginfo("-> Executing Linux Runner ('%s')" % str(cmd))
            subprocess.Popen(cmd)
        elif platform.system() == 'Windows':

            flags = 0
            flags |= 0x00000008  # DETACHED_PROCESS
            flags |= 0x00000200  # CREATE_NEW_PROCESS_GROUP
            flags |= 0x08000000  # CREATE_NO_WINDOW

            pkwargs = {
                'close_fds': True,  # close stdin/stdout/stderr on child
                'creationflags': flags,
            }
            cmd.insert(0, sys.executable)
            self.loginfo("-> Executing Windows Runner ('%s')" % str(cmd))
            P = subprocess.Popen(cmd, **pkwargs)

            pass

    def schedule_runner(self):
        # ORPHANED method - delete someday
        # self.loginfo(">>> Runner scheduling (%s) <<<" % self.execution_mode)
        if self._state == {}:
            never_ran = True

        else:
            never_ran = False
            start_time = iso_asdatetime(self._state['start_time'])
            end_time = iso_asdatetime(self._state['end_time'])
        pluginname = os.path.realpath(__file__)
        if self.execution_mode == 'agent_serial':
            execution_interval = timedelta(
                seconds=self.config.global_dict['execution_interval'])
            if never_ran or (self.get_now_as_dt() > start_time + execution_interval):
                if never_ran:
                    self.loginfo(
                        "Execution interval (%ds) for Runner is elapsed since last start." % (execution_interval.seconds))
                else:
                    self.loginfo(
                        "Execution interval (%ds) for Runner is elapsed since last start at %s" % (execution_interval.seconds, self._state['end_time']))
                    if self.is_running:
                        # IDEA: Controller can monitor its own log files. (WARN/ERROR)
                        self.logerror(
                            'Serial mode prohibits parallel Runner starts; there is ' +
                            'still one running since %s. ' %
                            localized_iso(self._state['start_time']))
                        self.loginfo("Either remove suites from execution list to save " +
                                     "execution time or increase the execution interval.")
                        return
                cmd = [pluginname, '--run']
                self.os_popen(cmd)

            else:
                # Idle...
                secs_to_execute = (
                    start_time + execution_interval - self.get_now_as_dt()).seconds
                self.loginfo("Nothing to do. Next Runner execution in %ds (interval=%ds)" % (
                    secs_to_execute, execution_interval.seconds))

        elif self.execution_mode == 'agent_parallel':
            # TBD
            pass
        else:
            # nothing to do her, execution is an external job
            pass

    def print_agent_output(self):
        '''Determines and prints the agent output; this is a JSON dict with two keys:
        - meta data:
          - static information like the robotmk version and encoding,
          - the runner's statefile (total execution time, cache time, executed suites etc.)
        - content of all suite statefiles as configured
        '''
        # self.loginfo(">>> Agent output generation <<<")
        output = []
        encoding = self.global_dict['agent_output_encoding']
        meta_data = {
            "encoding": encoding,
            "robotmk_version": ROBOTMK_VERSION,
        }
        self.logdebug("Reading the Runner statefile %s" %
                      self.statefile_path)
        self._state = self.read_state_from_file()
        meta_data.update(self._state)

        # Some keys from the runner state file should be overwritten with current values:
        meta_data.update({
            'robotmk_version': ROBOTMK_VERSION,
            'execution_mode': self.execution_mode}
        )

        self.loginfo(
            "Reading suite statefiles and encoding data (%s)..." % encoding)
        self.suites_state = self.check_suite_statefiles(encoding)
        if self.suites_state != None:
            for host in self.suites_state.keys():
                # discard empty dicts
                states = [
                    state for state in self.suites_state[host] if bool(state)]

                host_state = {
                    "runner": meta_data,
                    "suites": states,
                }
                json_serialized = json.dumps(host_state, sort_keys=False, indent=2)
                json_w_header = f'<<<robotmk:sep(0)>>>\n{json_serialized}\n'
                if host != "localhost":
                    json_w_header = f'<<<<{host}>>>>\n{json_w_header}\n<<<<>>>>\n'
                output.append(json_w_header)
        self.loginfo("Agent output printed on STDOUT")
        print(''.join(output))

    @property
    def global_dict(self):
        return self.config.cfg_dict['global']

    @property
    def suites_dict(self):
        return self.config.cfg_dict['suites']

    @property
    def outputdir(self):
        return self.global_dict['outputdir']

    def check_suite_statefiles(self, encoding):
        '''Check the state files of suites; encode specific keys'''
        states = defaultdict(list)
        self.loginfo("%d Suites to check: %s" % (len(self.suites_dict.keys()),
                                                 ', '.join(self.suites_dict.keys())))
        for suite in self.config.suite_objs(self.logger):
            # if (piggyback)host is set, results gets assigned to other CMK host
            host = suite.suite_dict.get('piggybackhost', 'localhost')
            self.logdebug("Reading statefile of suite '%s': %s" % (
                suite.id, suite.statefile_path))
            state = suite.read_state_from_file()

            if not bool(state):
                error_text = "Suite statefile not found - (seems like the suite did never run)"
                self.logwarn(error_text)

                state.update({
                    'id': suite.id,
                    'status': 'fatal',
                    'error': error_text
                })
            else:
                if state.get('rc', 0) >= 252:
                    state.update({
                        'status': 'fatal',
                        'error': 'Robot RC was >= 252. This is a fatal error. Robotmk got no XML/HTML to process. You should execute and test the suite manually.'
                    })
                else:
                    state.update({'status': 'nonfatal'})

                for k in self.keys_to_encode:
                    if k in state:
                        # Do not transfer HTML log if disabled in WATO
                        if k == 'htmllog' and self.global_dict['transmit_html'] == False:
                            state[k] = None
                        else:
                            content = self.read_file(state[k])
                            if k == 'xml':
                                # Remove any HTML content (embedded images) to not clutter the CMK multisite
                                content = xml_remove_html(content)
                                pass
                            state[k] = self.encode(
                                content,
                                suite.global_dict['agent_output_encoding'])
            states[host].append(state)
        if bool(states):
            return states
        else:
            return None

    @property
    def keys_to_encode(self):
        return ['xml', 'htmllog']

    def encode(self, data, encoding):
        data = data.encode('utf-8')
        if encoding == 'base64_codec':
            data_encoded = self.to_base64(data)
        elif encoding == 'zlib_codec':
            # zlib bytestream is base64 wrapped to avoid nasty bytes wihtin the
            # agent output. The check has first to decode the base64 "shell"
            data_encoded = self.to_zlib(data)
        elif encoding == 'utf_8':
            # nothing to do, already in utf8
            data_encoded = data
        else:
            # TODO: Catch the exception! (wrong encoding)!
            pass
        # as we are serializing the data to JSON, let's convert the bytestring
        # again back to UTF-8
        return data_encoded.decode('utf-8')            

    def to_base64(self, data):
        data_base64 = base64.b64encode(data)
        return data_base64

    # opens a file and returns the compressed content.
    # Caveat: to keep the zlib stream integrity, it must be converted to a
    # "safe" stream afterwards.
    # Reason: if there is a byte in the zlib stream which is a newline byte
    # by accident, Checkmk splits the byte string at this point - the
    # byte gets lost, stream integrity bungled.
    # Even if base64 blows up the data, this double encoding still saves space:
    # in:      692800 bytes  100    %
    # zlib:      4391 bytes    0,63 % -> compression 99,37%
    # base64:    5856 bytes    0,85 % -> compression 99,15%
    def to_zlib(self, data):
        data_zlib = zlib.compress(data, 9)
        data_zlib_b64 = self.to_base64(data_zlib)
        return data_zlib_b64

    def read_file(self, path, default=None):
        content = None
        try:
            with open(path, 'r', encoding='utf-8') as file:
                content = file.read()
                if len(content) == 0:
                    self.logwarn("File %s has no content, using defaults (%s)" % (
                        path, str(default)))
                    content = default
        except Exception as e:
            self.logwarn("Error while reading %s (%s); using default (%s)" % (
                path, e, str(default)))
            content = default
        return content


    def cleanup_logs(self):
        # cleanup logs which begin like this
        file_pattern = str(Path(self.outputdir).joinpath('robotframework_*'))
        if not 'log_rotation' in self.global_dict: 
            self.logwarn("robotmk.yml does not contain 'log_rotation' (you fiddled around, ehm?). Assuming default: 14")
            max_fileage = 14
        else: 
            max_fileage = int(self.global_dict['log_rotation'])
        self.logdebug("Logstate file retention: %d day(s)" % max_fileage)
        # and end with this
        file_regex = '.*_\d{10}_.*(log|output)\.(xml|html)'
        robot_logfiles = [file for file in glob.glob(file_pattern) if re.match(file_regex, file)]
        for item in robot_logfiles:
            if os.path.isfile(item):
                filedate = datetime.fromtimestamp(os.path.getmtime(item))
                if filedate < datetime.now() - timedelta(days = int(max_fileage)):
                    self.logdebug(f'Deleting old log file {item} (%s)...' % filedate.strftime('%Y.%m.%d %H:%M:%S'))
                    os.remove(item)

def xml_remove_html(content):
    xml = ET.fromstring(content)
    root = xml.find('./suite')
    imgmsg = [s for s in root.iter('msg') if 'html' in s.attrib]
    for s in root.iter('msg'):
        if 'html' in s.attrib:
            s.text = '(Robotmk has removed this HTML content for safety reasons)'
    content_wo_html = ET.tostring(
        xml, encoding='utf8', method='xml').decode()
    return content_wo_html


def localized_iso(iso):
    '''Convert a ISO formatted time string to the local tz

    Args:
        iso (string): ISO time string

    Returns:
        string: time string in local time zone
    '''
    return parser.isoparse(iso).astimezone()


def iso_asdatetime(iso):
    return parser.isoparse(iso)


def test_for_modules():
    try:
        global yaml
        import yaml
        global robot
        import robot
        import robot.rebot
        global mergedeep
        import mergedeep
        global parser
        from dateutil import parser
    except ModuleNotFoundError as e:
        print('<<<robotmk>>>')
        print(
            f'FATAL ERROR!: Robotmk cannot start because of a missing Python3 module (Error was: {str(e)})')
        print('Please execute: pip3 install robotframework pyyaml mergedeep python-dateutil')
        exit(1)

def main():
    test_for_modules()
    RMKPlugin.get_args()
    rmk = RMKCtrl()
    rmk.print_agent_output()
    rmk.loginfo("--- Quitting Controller, bye. ---")


if __name__ == '__main__':
    main()
else:
    # when imported as module
    import mergedeep
    import robot
    import yaml
    from dateutil import parser
