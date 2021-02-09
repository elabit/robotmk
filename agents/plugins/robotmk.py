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

# redirect stdout while testing: https://www.devdungeon.com/content/using-stdin-stdout-and-stderr-python

from pathlib import Path
from collections import defaultdict
import os
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

local_tz = datetime.utcnow().astimezone().tzinfo
ROBOTMK_VERSION = "v0.2.0"


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

    def __init__(self, calling_cls):
        self.calling_cls = calling_cls
        # merge I: combine the os and noarch defaults
        defaults_dict = self.__merge_defaults()
        # merge II: YML config overwrites the defaults
        robotmk_dict = self.read_robotmk_yml()
        robotmk_dict_merged_default = mergedeep.merge(robotmk_dict, defaults_dict)
        # merge III: environment vars overwrite the YML config
        envdict = self.read_env2dictionary()
        robotmk_dict_merged_env = mergedeep.merge(robotmk_dict_merged_default, envdict)
        
        self.cfg_dict = robotmk_dict_merged_env
        # now that YML and ENV are read, see if there is any suite defined. 
        # If not, the fallback is generate suite dict entries for every dir 
        # in robotdir. 
        if len(self.suites_dict) == 0:
            self.suites_dict = self.__suites_from_robotdirs()
        pass

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
        suites_dict = {
            suitedir.name: {
                'robotpath': suitedir.name
            } for suitedir in 
            Path(self.global_dict['robotdir']).iterdir()}
        return suites_dict

    @property
    def global_dict(self):
        return self.cfg_dict['global']

    @property
    def suites_dict(self):
        return self.cfg_dict['suites']

    @suites_dict.setter
    def suites_dict(self, suites_dict):
        self.cfg_dict['suites'] = suites_dict

    # @property
    # def suites(self):
    #     return list(self.suites_dict.keys())

    @property
    def suites(self):
        # suites = {id: RMKSuite(id, self.config) for id in self.config.suites}
        # return RMKSuites(self.cfg_dict)
        return [ RMKSuite(id, self.cfg_dict) for id in self.cfg_dict['suites']]


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
    def read_env2dictionary(cls, prefix='ROBOTMK_',
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

    def read_robotmk_yml(self):
        robotmk_yml = Path(self.get_robotmk_var(
            'agent_config_dir')).joinpath(
            self.get_robotmk_var('robotmk_yml'))
        if os.access(robotmk_yml, os.R_OK):
            self.calling_cls.logdebug(f'Reading configuration file {robotmk_yml}')
            #TEST: Reading a valid robotmk.yml
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
            #TEST: Valid config 100% from environment (-> Docker!)
            self.calling_cls.loginfo("No control file %s found. ")
            return {}


# class RMKSuites():
#     def __init__(self, cfg_dict):
#         self.suites = [
#             RMKSuite(id, cfg_dict)
#             for id in cfg_dict['suites']]
#         pass

#     def run(self):
#         #TODO: use this
#         '''Execute this suite
#         '''        
#         pass
        
#my-rmksuite
#TODO: for global and individual settings, add custom properties which try to 
#read both 
class RMKSuite():
    def __init__(self, id, cfg_dict):
        self.start_time = None
        self.end_time = None
        self.id = id
        # path is the only non-RF key in suite dict. Move this up
        self.robotpath = cfg_dict['suites'][id].pop('path', id)
        self.cfg_dict = {
            'global': cfg_dict['global'],
            'suite': cfg_dict['suites'][id]
        }
        self.statefile = RMKStatefile(self)

    def __str__(self):
        return self.id

    def update_filenames(self):
        now = int(time())
        suite_filename = "robotframework_%s_%s" % (self.id, str(now))
        #TODO: Make it possible to use global and suite config 
        self.suite_dict.update({
            'outputdir':  self.global_dict['outputdir'],
            'output': f'{suite_filename}_output.xml',
            'log':    f'{suite_filename}_log.html',
            'report': f'{suite_filename}_report.html',
        })

    def clear_filenames(self): 
        '''Reset the log file names if Robot Framework exited with RC > 250
        The files presumed to exist do not in this case. 
        '''        
        self.outfile_htmllog = None
        self.outfile_htmlreport = None
        self.outfile_xml = None

    def robotize_variables(self): 
        # Preformat Variables to meet the Robot API requirement 
        # --variable name:value --variable name2:value2 
        # => ['name:value', 'name2:value2'] (list of dicts to list of k:v)
        if 'variable' in self.suite_dict: 
            self.suite_dict['variable'] = list(
                map(
                    lambda x: f'{x[0]}:{x[1]}',
                    self.suite_dict['variable'].items()
                ))

    def start(self):
        self.robotize_variables()
        self.update_filenames()
        self.start_time = datetime.now(tz=local_tz)
        rc = robot.run(
            self.path,
            **self.suite_dict)
        self.rc = rc
        self.end_time = datetime.now(tz=local_tz)
        self.runtime = (self.end_time - self.start_time).total_seconds()
        return rc

    @property
    def path(self): 
        return Path(self.global_dict['robotdir']).joinpath(self.robotpath)

    @property
    def suite_dict(self):
        return self.cfg_dict['suite']

    @property
    def global_dict(self):
        return self.cfg_dict['global']

    @property
    def outfile_xml(self): 
        if not self.suite_dict['output'] is None: 
            return str(Path(self.global_dict['outputdir']).joinpath(
            self.suite_dict['output']))
        else: 
            return None

    @property
    def outfile_htmllog(self): 
        if not self.suite_dict['log'] is None: 
            return str(Path(self.global_dict['outputdir']).joinpath(
            self.suite_dict['log']))
        else: 
            return None

    @property
    def outfile_htmlreport(self): 
        if not self.suite_dict['report'] is None: 
            return str(Path(self.global_dict['outputdir']).joinpath(
            self.suite_dict['report']))
        else: 
            return None

    @outfile_xml.setter
    def outfile_xml(self, text): 
        self.suite_dict['output'] = None

    @outfile_htmllog.setter
    def outfile_htmllog(self, text): 
        self.suite_dict['log'] = None

    @outfile_htmlreport.setter
    def outfile_htmlreport(self, text): 
        self.suite_dict['report'] = None




#my-rmkstatefile
class RMKStatefile():
    def __init__(self, suite):
        self.suite = suite
        self.data = None
        
    def read(self): 
        '''Read the suite statefile and supplement with staleness info
        Returns:
            dict: Status data
        '''        
        try: 
            with open(self.path) as statefile:
                self.data = json.load(statefile) 
        except IOError: 
            pass
            #TODO: ERROR

        self.data['result_age'] = self.age.seconds
        self.data['result_overdue'] = self.overdue
        self.data['result_is_stale'] = self.is_stale()
        return self.data


    def is_stale(self):
        return self.age.seconds > int(self.cache_time)

    @property
    def age(self):
        '''Return the seconds since last execution and now()
        Returns:
            diff: timedelta
        '''        
        if self.data is None: 
            self.data = self.read()
        age = datetime.now(timezone.utc) - parser.isoparse(
            self.data['last_end_time'])
        return age

    @property
    # how many seconds the result is older than the cache
    def overdue(self): 
        return (datetime.now(timezone.utc) - \
            parser.isoparse(self.data['last_end_time']) - \
            timedelta(seconds=int(self.cache_time))).seconds

    @property
    def cache_time(self): 
        return self.data['cache_time']

    @property
    def mtime(self):
        '''Returns:
            [int]: mtime of the suite's statefile. 0 if not present.'''  
        try:
            mtime = datetime.fromtimestamp(
                os.path.getmtime(str(self.path)))
        except Exception:
            # file not found etc.
            mtime = -1
        return mtime

    @property
    def path(self):
        filename = f'robotmk_{self.suite.id}.json'
        return Path(self.suite.global_dict['outputdir']).joinpath(filename)

    def write(self):
        '''Writes the statefile content for a executed suite'''
        result_dict = {
            "id": self.suite.id, 
            "last_start_time": self.suite.start_time.isoformat(), 
            "last_end_time": self.suite.start_time.isoformat(),
            "cache_time": self.suite.global_dict['cache_time'],
            "runtime": self.suite.runtime,
            "last_rc": self.suite.rc,
            "xml": self.suite.outfile_xml,
            "htmllog": self.suite.outfile_htmllog,
        }
        with open(self.path, 'w', encoding='utf-8') as outfile: 
            json.dump(result_dict, outfile, indent=2, sort_keys=False)
        


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
        self.__setup_logging(calling_cls=self, verbose=self.cmdline_args.verbose)
        self.loginfo("="*20 + " START " + "="*20)
        self.config = RMKConfig(calling_cls=self)

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
                If called in 'plugin mode', robotmk executes Robot Framework suites. With "--run", 
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
        parser.add_argument(
            '--run',
            '-r',
            dest='suites',
            const='all',
            default=None,
            action='store',
            nargs='?',
            type=str,
            help="""Plugin mode. Runs all Robot Framework suites as configured in robotmk.yml.
                    Suite IDs can be given as comma separated list to restrict execution.
                    Suites are executed serially, one by one.""")
        parser.add_argument('--verbose',
                            '-v',
                            default=False,
                            action='store_true',
                            help="""Print the RobotMK log to console.""")
        cls.cmdline_args = parser.parse_args()

    
    def __setup_logging(self, calling_cls, verbose=False):
        if self._DEFAULTS['noarch']['logging']:
            instance_name = calling_cls.__class__.__name__
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
        self.logger.warn(text)

    @asinstance
    def logerror(self, text):
        self.logger.error(text)



#class RobotMK >>>

#my-rmkplugin
class RMKPlugin(RobotMK):
    def __init__(self): 
        super().__init__()
        pass

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
        suites_cmdline = [ x.strip() for x in suites_cmdline.split(',')]
        if not(len(suites_cmdline) == 1 and suites_cmdline[0] == "all"):
            # Suites given as arg but no cfg
            suites_inarg_notincfg = [suite for suite in suites_cmdline 
                                 if suite not in self.config.suites]
            if len(suites_inarg_notincfg) > 0:
                self.loginfo("(+) Adding suites: " +
                             f"'{','.join(suites_inarg_notincfg)}' " + 
                             "(not in cfg, but in arguments; will start with defaults.)")
                suites_inarg_notincfg_dict = {
                    suiteid: {
                        'path': suiteid
                    } for suiteid in suites_inarg_notincfg}

            # Remove suites from cfg which are not given as argument
            cfgsuites = self.config.suites
            for suite in cfgsuites:
                if suite not in suites_cmdline: 
                    self.loginfo(f"(-) Skipping suite '{suite}'' (in cfg, NOT in arguments)")
                    self.config.suites_dict.pop(suite, None) 
                else: 
                    self.loginfo(f"( ) Adding suite '{suite}' (in cfg and in arguments)")

            self.config.suites_dict.update(suites_inarg_notincfg_dict)
        


    #my-startsuites
    def start_suites(self, suites_cmdline):
        self.update_suites2start(suites_cmdline)
        suites = self.config.suites
        self.loginfo(
            ' => Suites to start: %s' % ', '.join([str(s) for s in suites]))
        tic = datetime.now(timezone.utc)
        for suite in suites:
            id = suite.id
            self.loginfo(f"---------- Suite: {id} ----------")
            self.logdebug(f'Starting suite')
            rc = suite.start()
            self.loginfo(f'Suite finished with RC {rc} after {suite.runtime} sec')
            if rc > 250: 
                self.logerror('RC > 250 = Robot exited with fatal error. There are no logs written.')
                self.logerror('Please run the robot command manually to debug.')
                suite.clear_filenames()
            self.loginfo(f'Writing statefile {suite.statefile.path}')
            suite.statefile.write()
        tac = datetime.now(timezone.utc)
        deltatime = tac - tic
        self.write_statefile(suites, deltatime)

    def write_statefile (self, suites, deltatime):
        runtime_total = deltatime.total_seconds()
        runtime_suites = sum([s.runtime for s in suites])
        json_dict = {
            'runtime_total': runtime_total,
            'runtime_suites': runtime_suites,
            'runtime_robotmk': runtime_total - runtime_suites,
            'suites': [(s.id, s.runtime) for s in suites]
        }
        with open(Path(
            self.config.global_dict['outputdir']).joinpath(
            'robotmk_plugin.json'), 'w',
            encoding='utf-8') as plugin_statefile:
            json.dump(json_dict, plugin_statefile, indent=2)
        pass


class RMKCtrl(RobotMK):
    #TODO: Cleanup the XML!

    keys_to_encode = ['xml', 'htmllog']
    header = '<<<robotmk>>>'

    def __init__(self):
        super().__init__()

    def agent_output(self): 
        output = []
        encoding = self.config.global_dict['agent_output_encoding']
        metadata = {
            "encoding": encoding, 
            "robotmk_version": ROBOTMK_VERSION,
            "runtime": "FIXME"
        }
        allstates = self.check_states(encoding)
        for host in allstates.keys():  
            states = allstates[host]  
            host_state = {
                "metadata": metadata, 
                "suites": states,
            }
            json_serialized = json.dumps(host_state, sort_keys=False, indent=2)
            json_w_header = f'<<<robotmk:sep(0)>>>\n{json_serialized}\n'
            if host != "localhost": 
                json_w_header = f'<<<<{host}>>>>\n{json_w_header}\n<<<<>>>>\n'
            output.append(json_w_header)
        return ''.join(output)


    def check_states(self, encoding):
        '''Check the state files of suites; encode specific keys'''  
        states = defaultdict(list)
        for suite in self.config.suites:
            # if (piggyback)host is set, results gets assigned to other CMK host
            host = suite.suite_dict.get('host', 'localhost')
            state = suite.statefile.read()
            for k in self.keys_to_encode: 
                if bool(state[k]):  
                    content = self.read_file(state[k])
                    state[k] = self.encode(
                        content, 
                        suite.global_dict['agent_output_encoding'])
            states[host].append(state)
        return states

    def encode(self, data, encoding):
        data = data.encode('utf-8')
        if encoding == 'base64_codec':
            data_encoded = self.to_base64(data)
        elif encoding == 'zlib_codec':
            # zlib bytestream is base64 wrapped to avoid nasty bytes wihtin the
            # agent output. The check has first to decode the base64 "shell"
            data_encoded = self.to_zlib(data)
        else: 
            #TODO: ERROR wrong encoding!
            pass
        # as we are serializing the data to JSON, let's convert the bytestring
        # again back to UTF-8
        return data_encoded.decode('utf-8')

    def to_base64(self, data):
        data_base64  = base64.b64encode(data)
        return data_base64

    # opens the Robot XML file and returns the compressed xml result.
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
        # As only the agent output is compressed (not the header), the check will see one very long byte stream. 
        # TODO: Remove the separator from check
        data_zlib = zlib.compress(data, 9)
        data_zlib_b64 = self.to_base64(data_zlib)
        return data_zlib_b64

    def read_file(self, path): 
        with open(path, 'r', encoding='utf-8') as file: 
            content = file.read()
        return content

def test_for_modules():
    try:
        global yaml
        import yaml
        global robot
        import robot
        global mergedeep
        import mergedeep
        global parser
        from dateutil import parser
    except ModuleNotFoundError as e:
        logger.error(f'Could not start because of a missing module: {str(e)}')
        exit(1)


if __name__ == '__main__':
    test_for_modules()
    RobotMK.get_args()
    cmdline_suites = RobotMK.cmdline_args.suites
    instance = None
    if cmdline_suites is None:
        instance = RMKCtrl() 
        data = instance.agent_output()
        print(data)
        #This is the point where the controleler will not only monitor state
        #files, but also start new plugin executions!
    else:
        instance = RMKPlugin()
        instance.start_suites(cmdline_suites)
    instance.loginfo("="*20 + " FINISHED " + "="*20)
else: 
    # when imported as module
    import mergedeep
    import robot
    import yaml
    from dateutil import parser
