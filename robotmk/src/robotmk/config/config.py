"""This module provides a simple way to read configuration from different sources:
- YML file
- variables file
- environment variables

There is a special order in which the sources are read:
- 1. OS defaults for each supported OS (Linux, Windows)
- 2. config from YML file, either
    - the default config file (robotmk.yml)
    OR
    - a custom config file (given as parameter --yml)
- 3. variables from
    - variable file (given as parameter --vars)
    AND
    - environment variables (ROBOTMK_*)

| context       | yml    | vars |
| ---           | ---    | ---  |
| agent         | X      |      |
| specialagent  |        | X    |
| suite         | X      | X    |


"""

# TODO: add a mode to dump environment
# mypy: disable-error-code="import, var-annotated, return-value, valid-type, operator, index, assignment, return"

import hashlib
import json
import os
import re
from collections import defaultdict
from functools import wraps

# from collections import namedtuple
from pathlib import Path
from typing import Union

import yaml
from mergedeep import merge, Strategy

from robotmk.main import DIR_SUBPATHS

# TODO: add YML config validation
# from .yml import RobotmkConfigSchema
# ["%s:%s" % (v, os.environ[v]) for v in os.environ if v.startswith("ROBO")]


# def default_dict_constructor(loader, node):
#     return defaultdict(loader.construct_mapping(node))


# yaml.add_constructor(
#     "tag:yaml.org,2002:python/object/apply:collections.defaultdict",
#     default_dict_constructor,
# )


def add_path_prefix(method):
    """Decorator function for the get() method.

    If the developer has set common.path_prefix (ROBOTMK_common_path__prefix),
    then this folder becomes the path prefix for a standard set of folder inside.
    The path settings for cfgdir, logdir etc. are ignored in that case.
    See main.py, ref 5aa211
    """

    @wraps(method)
    def wrapper(self, *args, **kwargs):
        value = method(self, *args, **kwargs)
        dir_el = args[0].split(".")[-1]

        if dir_el in [
            "cfgdir",
            "logdir",
            "tmpdir",
            "resultdir",
            "robotdir",
        ]:
            prefix = self.get("common.path_prefix", None)
            if prefix:
                value = str(Path(prefix) / DIR_SUBPATHS[dir_el])
            else:
                # no prefix given, so we use the value as is
                pass
        return value

    return wrapper


class Config:
    def __init__(self, envvar_prefix: str = "ROBOTMK", initdict: dict = {}):
        self.envvar_prefix = envvar_prefix
        self.envvar_prefix_lower = envvar_prefix.lower()
        if initdict:
            self.default_cfg = initdict
        else:
            self.default_cfg = {}
        self.yml_config = {}
        self.env_config = {}
        # this is a dict of all config values that were added by the user.
        # they are applied last and can overwrite any other config values.
        self.added_config = {}

    def __iter__(self):
        # TODO: iterator added afterwards - where is it useful?
        for key in self.configdict:
            yield key, self.get(key)

    def __translate_keys(self, name: str) -> list:
        """Translate a key name to a list of keys and respect shorthand 'suitecfg'."""
        keys = name.split(".")
        if keys[0] == "suitecfg":
            suitename = self.configdict["common"]["suiteuname"]
            keys[:1] = ["suites", suitename]
        return keys

    @add_path_prefix
    def get(self, name: str, default=None, asdict=False) -> str:
        """Get a value from the object with dot notation.

        Shorthands:
        - 'suitecfg' can be used for 'suites.<suiteuname>'.
        - 'basic_cfg' can be used for the basic config dict.

        Examples:
            cfg.get("common.logdir")
            cfg.get("suitecfg.run.rcc")
            cfg.get("basic_cfg.common.logdir") -> returns logdir value from the configuration
        """
        keys = self.__translate_keys(name)
        if keys[0] == "basic_cfg":
            m = self.basic_cfg
            keys = keys[1:]
        else:
            m = self.configdict
        # prev = self.configdict
        try:
            for key in keys:
                # prev = m
                # prev_k = key
                m = m.get(key, {})
            if m:  # non-empty value
                if type(m) is dict:
                    if asdict:
                        return m
                    else:
                        return Config(initdict=m)
                else:
                    return m
            else:  # empty dict
                return default

        except:
            return default

    def set(self, name: str, value: any) -> None:
        """Set a value in the object with dot notation.

        Shorthand 'suitecfg' can be used for 'suites.<suiteuname>'.

        Example:
            cfg.set("common.logdir", "/foo/log")
            cfg.set("suitecfg.run.rcc", False)
        """
        keys = self.__translate_keys(name)
        cur_dict = self.added_config
        for key in keys[:-1]:
            if not key in cur_dict:
                cur_dict[key] = {}
            cur_dict = cur_dict[key]

        cur_dict[keys[-1]] = value

    def asdict(self):
        """Returns the config as a dict."""
        return self.configdict

    def suite_cfghash(self, suiteuname) -> str:
        """Returns a hash of the common + suite config (passed as arg) to identify a possible change.

        Used in the scheduler and sequencer."""
        common_cfg = {"common": self.get("common").asdict()}
        suite_cfg = {"suites": self.get("suites.%s" % suiteuname).asdict()}
        cfg = merge({}, common_cfg, suite_cfg)
        return hashlib.sha256(json.dumps(cfg).encode("utf-8")).hexdigest()

    @property
    def configdict(self):
        """This property represents all config dicts merged.

        It consist of the following dicts:
        - basic_cfg = predefiend from start
            - default_cfg
            - yml_config
            - env_config
        - added_config = added at runtime

        """

        # https://mergedeep.readthedocs.io/en/latest/#merge-strategies
        # Collection values are merged additively. Others get replaced.
        # The last/rightmost dictionary has the highest priority.
        return merge(
            {},
            self.basic_cfg,  # Config set by OS default values, YML file, environment variables and/or variables file
            self.added_config,  # Config changed/added at runtime (config.set() method)
            strategy=Strategy.ADDITIVE,
        )

    @configdict.setter
    def configdict(self, cfg: dict):
        """This setter allows to write a whole new config dict. It is used e.g. when the scheduler creates
        job object for each suite."""
        self.default_cfg = cfg

    @property
    def basic_cfg(self):
        """Returns the basic config dict."""
        return merge(
            {},
            self.default_cfg,  # Config set by OS default values
            self.yml_config,  # Config loaded from YML file
            self.env_config,  # Config loaded from environment variables and/or variables file
            strategy=Strategy.ADDITIVE,
        )

    # 1. Defaults (common/OS specific)
    def set_defaults(self, os_defaults: dict = None) -> None:
        """Sets the defaults for the current OS."""
        self.default_cfg["common"] = {}
        if os_defaults:
            self.default_cfg["common"].update(os_defaults["common"])
        if os.name in os_defaults:
            self.default_cfg["common"].update(os_defaults[os.name])

    # 2. YML
    def read_yml_cfg(self, path=None, must_exist=True):
        """Reads a YML config, either default or custom."""
        if path is None:
            # Linux default: /etc/check_mk/robotmk.yml
            # Windows default: C:\Program Data\check_mk\agent\config\robotmk.yml
            ymlfile = Path(self.get("common.cfgdir")) / self.get("common.robotmk_yml")
        else:
            ymlfile = Path(path)
            # a custom file path should always exist
            must_exist = True
        if must_exist and not ymlfile.exists():
            raise FileNotFoundError(f"YML config file not found: {ymlfile}")
        else:
            # try to read the file
            config = {}
            try:
                with open(ymlfile, "r") as f:
                    config = yaml.load(f, Loader=yaml.FullLoader)
            except Exception as e:
                raise e

            self.yml_config = config

    # 3. variables (env AND! file)
    def read_cfg_vars(self, path=None):
        """Read ROBOTMK variables from file and/or environment.

        Environment vars have precedence over file vars."""

        filevars = self._filevar2dict(path)
        envvars = self._envvar2dict()
        # a dict with still flat var names
        vars = merge(filevars, envvars)
        # convert flat vars to nested dicts
        vdict = defaultdict(dict)
        for k, v in vars.items():
            # iterate over each variable and convert it to a nested data structure
            vdict = self._var2cfg(k, v, vdict)
        self.env_config = merge({}, self.env_config, vdict, strategy=Strategy.ADDITIVE)

    def _envvar2dict(self) -> dict:
        """Returns all environment variables starting with the ROBOTMK prefix.

        Example:
        {"ROBOTMK_foo_bar": "baz",
         "ROBOTMK_foo_baz": "bar"}
        """
        vardict = {}
        for k, v in os.environ.items():
            if k.lower().startswith(self.envvar_prefix_lower):
                # check if the value is a boolean and convert it to a boolean
                if v.lower() in ("true", "false"):
                    v = v.lower() == "true"
                vardict[k.lower()] = v
        return vardict

    def _filevar2dict(self, file) -> dict:
        """Returns all variables from a given file (strips 'set' and 'export' statements).

        Example:
        {"ROBOTMK_foo_bar": "baz",
         "ROBOTMK_foo_baz": "bar"}
        """
        r = {}
        if file:
            try:
                with open(file, "r") as f:
                    for line in f:
                        line = line.strip()
                        # Ignore empty lines and lines starting with "#" (comments)
                        if line.strip() and not line.strip().startswith("#"):
                            if line.startswith("export ") or line.startswith("set "):
                                line = line.partition(" ")[2]
                            # Split each line into a key-value pair
                            key, value = line.strip().split("=")
                            if key.lower().startswith(self.envvar_prefix_lower):
                                r[key] = value
            except Exception as e:
                raise FileNotFoundError(f"Could not read environment file: {file}")
        return r

    # def validate(self, schema: RobotmkConfigSchema):
    #     """Validates the whole config according to the given context schema."""

    #     schema = RobotmkConfigSchema(self.configdict)
    #     if not schema.validate():
    #         raise ValueError(f"Config is invalid: {schema.error}")

    @staticmethod
    def partition_at_digit(s):
        """Divides the string at the first digit and returns a list of the
        left part, the digit and the right part.

        The digit indicates the index in the list."""
        m = re.search("_\d+", s)
        if m:
            return s[: m.start()], int(s[m.start() + 1 : m.end()]), s[m.end() + 1 :]
        else:
            return [s, None, None]

    @staticmethod
    def split_varstring(s):
        """Helper function to reconstruct the nested key path of a dict from
        a varname. The keys are separated by "_", a double underscore protects the string from splitting.
        """
        keys = []
        starti = 0
        i = 0
        while i < len(s):
            poschar = s[i]
            if poschar == "_" or i == len(s) - 1:
                if len(s) > i + 1 and s[i + 1] == "_":
                    # not at and and double underscore in front of us!
                    # skip next underscore and continue until next SINGLE underscore
                    i += 2
                    continue
                else:
                    # Single underscore in front or last piece; add current piece to list
                    # (replace __ by _) and start a new one
                    if len(s) > i + 1:
                        # last piece
                        last_single_usc_index = i
                    else:
                        # not last piece
                        last_single_usc_index = i + 1
                    piece = s[starti:last_single_usc_index].replace("__", "_")
                    keys.append(piece)
                    starti = i + 1
            else:
                # Add the current character to the current piece
                pass
            i += 1
        return keys

    def uscore_str2dict(string, value, vdict=None):
        """Converts a variable string to a nested dict.

        Digit inside the string indicate the index in the list."""
        # curdict is a moving reference to the current dict
        cur_dict = vdict

        (s_left, s_list_index, s_right) = Config.partition_at_digit(string)
        # list of keys "left" of the list index (if any)
        left_keys = Config.split_varstring(s_left)
        for ki, key in enumerate(left_keys):
            # descend now key by key until we reach the leaf key. cur_dict gets always
            # moved forward to have a reference on the key we want to change.
            if ki < len(left_keys) - 1:
                if not key in cur_dict:
                    cur_dict[key] = defaultdict(dict)
                cur_dict = cur_dict[key]
            else:
                # we have reached the leaf key of left side, just before the index.
                if not s_list_index is None:  # list index
                    if (
                        not key in cur_dict
                    ):  # key not present, create all (empty) list items
                        cur_dict[key] = [defaultdict(dict)] * (s_list_index + 1)
                    else:
                        if (
                            len(cur_dict[key]) < s_list_index + 1
                        ):  # key present, but not enough list items
                            cur_dict[key] = cur_dict[key] + [defaultdict(dict)] * (
                                s_list_index + 1 - len(cur_dict[key])
                            )
                    if not s_right:  # list entry without subkeys
                        cur_dict[key][s_list_index] = value
                    else:
                        # dict inside the list!
                        # call the function recursively to descend into the dict. We also
                        # pass the current list item as cur_dict, so that the function can
                        # modify it.
                        subdict = Config.uscore_str2dict(
                            s_right, value, cur_dict[key][s_list_index]
                        )
                        # assign the modified subdict to the list item
                        cur_dict[key][s_list_index] = subdict
                else:
                    # simple value assignment. Make sure that values of numbers are not stored as strings.
                    if isinstance(value, str) and value.isdigit():
                        value = int(value)
                    cur_dict[key] = value

        return vdict

    def _var2cfg(self, o_varname, value, vdict) -> dict:
        """Helper function to convert a variable to a dict/list/value entry and assigns the value.

        Example:
            - ROBOTMK_foo_bar_0_baz = "bar"
                will be converted to:
                {"foo": {"bar": [{"baz": "bar"}]}}
            - ROBOTMK_foo__bar_0_baz = "bar
                will be converted to:
                {"foo_bar": [{"baz": "bar"}]}
        """

        # Remove the ROBOTMK_ prefix
        varname = o_varname.replace(self.envvar_prefix_lower + "_", "")
        Config.uscore_str2dict(varname, value, vdict)
        return vdict

    def dotcfg_to_env(self, dotstrings: dict, environ: dict):
        """Converts a dict of dotconfigs to an environment variable and assign it to environ.

        This does not affect the current config at all; it just uses a temporary config object.
        Example:
            {"common.logdir": "foo",
             "common.loglevel": "debug"}
        """

        tmp_cfg = Config()
        # As we are using a temp config object, the suiteuname must be set manually to access the
        # correct section with the shorthand "suitecfg"
        tmp_cfg.set("common.suiteuname", self.get("common.suiteuname"))
        for k, v in dotstrings.items():
            tmp_cfg.set(k, v)
        self.cfg_to_environment(tmp_cfg.configdict, environ=environ)

    def cfg_to_environment(self, d, prefix="", environ=None):
        """Converts a given dict to environment variables.

        If environ is given, the environment variables are added to the given dict.
        Otherwise the environment variables are added to the current environment.

        The conversion rules are:
        - there is no case conversion
        - underscores within key names are replaced by double underscores
        - the prefix is added to the environment variable name
        - dicts are converted to nested environment variables
        - lists get a number appended to their key name"""

        if isinstance(d, dict):  # DICT conversion
            for key, value in d.items():
                safe_key = key.replace("_", "__")
                new_prefix = f"{prefix}_{safe_key}"
                self.cfg_to_environment(value, prefix=new_prefix, environ=environ)
        elif isinstance(d, list):  # LIST conversion
            for i, item in enumerate(d):
                new_prefix = f"{prefix}_{i}"
                self.cfg_to_environment(item, prefix=new_prefix, environ=environ)
        else:  # VALUE conversion
            varname = f"{self.envvar_prefix}{prefix}"
            if isinstance(d, bool):
                # convert bools to lower case strings
                d = str(d).lower()
            print(f"{varname}={d}")
            if environ is None:
                # add variables to current environment
                os.environ[varname] = str(d)
            else:
                # add variables to given environment
                environ[varname] = str(d)

    def to_yml(self, file=None) -> Union[str, None]:
        """Dumps the config to a file or returns it."""
        if file:
            try:
                with open(file, "w") as f:
                    yaml.dump(self.configdict, f)
            except Exception as e:
                print(f"Could not write to file {file}: {e}")
                return None
        else:
            return yaml.dump(
                self.configdict, sort_keys=False, indent=4, default_flow_style=False
            )
