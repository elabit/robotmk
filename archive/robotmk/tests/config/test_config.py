# mypy: disable-error-code="import"
import os

import yaml

from robotmk.config.config import Config

cwd = os.path.dirname(__file__)
robotmk_yml = os.path.join(cwd, "robotmk.yml")
robotmk_env = os.path.join(cwd, "robotmk.env")


def test_suitecfg_shorthand():
    """Tests the 'suitecfg' shorthand which allows to set/get the config of the current
    suite by using the 'suitecfg' key."""
    cfg = Config()
    cfg.set("common.suiteuname", "foosuite")
    cfg.set("suitecfg.foo.bar", "baz")
    assert cfg.configdict["suites"]["foosuite"]["foo"]["bar"] == "baz"
    assert cfg.get("suitecfg.foo.bar") == "baz"


def test_path_prefix():
    os.environ[
        "ROBOTMK_common_path__prefix"
    ] = "/home/simonmeggle/Documents/01_dev/rmkv2/agent"

    cfg = Config()
    # read variables from environment
    cfg.read_cfg_vars(path=None)
    assert (
        cfg.get("common.logdir")
        == "/home/simonmeggle/Documents/01_dev/rmkv2/agent/log/robotmk"
    )
    assert (
        cfg.get("common.tmpdir")
        == "/home/simonmeggle/Documents/01_dev/rmkv2/agent/tmp/robotmk"
    )
    assert (
        cfg.get("common.robotdir")
        == "/home/simonmeggle/Documents/01_dev/rmkv2/agent/robots"
    )


def test_env2cfg_values():
    """Tests the conversion of environment variables to a config dictionary."""
    # simple values
    os.environ["ROBOTMK_suites_foo1"] = "qux"
    os.environ["ROBOTMK_common_log__level"] = "INFO"
    os.environ["ROBOTMK_common_suiteuname"] = "foo_suite"
    os.environ["ROBOTMK_suites_foo__suite_my__value"] = "qux"
    os.environ["ROBOTMK_suites_foo__suite_run_rcc"] = "false"
    cfg = Config()
    # read variables from environment
    cfg.read_cfg_vars(path=None)
    assert str(cfg.configdict["common"]["log_level"]) == "INFO"
    assert str(cfg.configdict["common"]["suiteuname"]) == "foo_suite"
    assert str(cfg.configdict["suites"]["foo_suite"]["my_value"]) == "qux"
    assert cfg.configdict["suites"]["foo_suite"]["run"]["rcc"] == False


def test_env2cfg_dicts():
    """Tests the conversion of environment variables to a config dictionary."""
    # # dicts
    os.environ["ROBOTMK_suites_foo__suite_my__dict_foo"] = "one"
    os.environ["ROBOTMK_suites_foo__suite_my__dict_bar"] = "two"
    os.environ["ROBOTMK_suites_foo__suite_my__dict_baz"] = "three"
    cfg = Config()
    # read variables from environment
    cfg.read_cfg_vars(path=None)

    assert cfg.configdict["suites"]["foo_suite"]["my_dict"] == {
        "foo": "one",
        "bar": "two",
        "baz": "three",
    }


def test_env2cfg_lists():
    """Tests the conversion of environment variables to a config dictionary."""
    # # # lists
    os.environ["ROBOTMK_suites_foo__suite_my__list_0"] = "one"
    os.environ["ROBOTMK_suites_foo__suite_my__list_1"] = "two"
    os.environ["ROBOTMK_suites_foo__suite_my__list_2"] = "three"
    cfg = Config()
    # read variables from environment
    cfg.read_cfg_vars(path=None)
    assert cfg.configdict["suites"]["foo_suite"]["my_list"] == [
        "one",
        "two",
        "three",
    ]


def test_env2cfg_listofdicts():
    """Tests the conversion of environment variables to a config dictionary."""
    # list of dicts
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_foo"] = "one"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_bar"] = "two"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_baz"] = "three"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_foo"] = "one"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_bar"] = "two"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_baz"] = "three"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_foo"] = "one"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_bar"] = "two"
    os.environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_baz"] = "three"
    cfg = Config()
    # read variables from environment
    cfg.read_cfg_vars(path=None)
    assert cfg.configdict["suites"]["foo_suite"]["my_list_of_dict"] == [
        {"foo": "one", "bar": "two", "baz": "three"},
        {"foo": "one", "bar": "two", "baz": "three"},
        {"foo": "one", "bar": "two", "baz": "three"},
    ]


def test_defaults():
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2}})
    assert cfg.configdict["common"]["a"] == 1
    assert cfg.configdict["common"]["b"] == 2


def test_read_yml_cfg():
    """Tests if parts of the default config are overwritten by the yml config.
    The default config sets a=1 and b=2, yml changes b to 3."""
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2}})
    cfg.read_yml_cfg(path=robotmk_yml)
    # unchanged
    assert cfg.configdict["common"]["a"] == 1
    # changed
    assert cfg.configdict["common"]["b"] == 3


def test_read_var_env_cfg():
    """Tests if parts of the default config are overwritten by
    - YML config
    - and then again overwritten by environment variables
    The default config sets a=1, b=2, c=3.
    Yml changes b to 3.
    Env changes c to 4."""
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2, "c": 3}})
    cfg.read_yml_cfg(path=robotmk_yml)
    # TODO: remove this line, it is only for testing
    os.environ["ROBOTMK_common_c"] = "4"
    os.environ["ROBOTMK_common_cc"] = "4"
    os.environ["ROBOTMK_common_cff"] = "4"
    os.environ["ROBOTMK_common_chhh"] = "4"
    # read variables from environment
    cfg.read_cfg_vars(path=None)
    # unchanged
    assert str(cfg.configdict["common"]["a"]) == "1"
    assert str(cfg.configdict["common"]["b"]) == "3"
    # changed
    assert str(cfg.configdict["common"]["c"]) == "4"


def test_read_var_file_cfg():
    """Tests if parts of the default config are overwritten by
    - YML config
    - variables in a file
    - and then again overwritten by env vars
    The default config sets a=1, b=2, c=3.
    Yml changes b to 3.
    File changes c to 5.
    Env changes c to 55."""
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2, "c": 3}})
    cfg.read_yml_cfg(path=robotmk_yml)
    os.environ["ROBOTMK_common_c"] = "55"
    cfg.read_cfg_vars(path=robotmk_env)
    # unchanged
    assert str(cfg.configdict["common"]["a"]) == "1"
    assert str(cfg.configdict["common"]["b"]) == "3"
    # changed
    assert str(cfg.configdict["common"]["c"]) == "55"


def test_read_var_file_added_cfg():
    """Tests if parts of the default config are overwritten by
    - YML config
    - variables in a file
    - env vars
    - and then again overwritten by added vars
    The default config sets a=1, b=2, c=3.
    Yml changes b to 3.
    File changes c to 5.
    Env changes c to 55.
    Added var changes c to 6."""
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2, "c": 3}})
    cfg.read_yml_cfg(path=robotmk_yml)
    os.environ["ROBOTMK_common_c"] = "55"
    cfg.read_cfg_vars(path=robotmk_env)
    cfg.set("common.c", 66)
    cfg.set("common.d.e.f.g", 77)
    # unchanged
    assert str(cfg.configdict["common"]["a"]) == "1"
    assert str(cfg.configdict["common"]["b"]) == "3"
    # changed
    assert str(cfg.configdict["common"]["c"]) == "66"
    assert str(cfg.configdict["common"]["d"]["e"]["f"]["g"]) == "77"


def test_config_to_yml():
    """Tests if the config can be dumped to a valid YML string"""
    cfg = Config()
    cfg.set_defaults({"common": {"a": 1, "b": 2, "c": 3}})
    cfg.read_yml_cfg(path=robotmk_yml)
    os.environ["ROBOTMK_common_c"] = "4"
    os.environ["ROBOTMK_foo_bar_x"] = "44"
    cfg.read_cfg_vars(path=None)
    # read variables from a file
    cfg.read_cfg_vars(path=robotmk_env)
    # unchanged
    assert str(cfg.configdict["common"]["a"]) == "1"
    assert str(cfg.configdict["common"]["b"]) == "3"
    # changed
    assert str(cfg.configdict["common"]["c"]) == "4"
    yml_str = cfg.to_yml()
    yml = yaml.load(yml_str, Loader=yaml.Loader)
    assert str(yml["foo"]["bar"]["x"]) == "44"


def test_envvar2dict():
    """Tests if only env vars with the prefix ROBOTMK_ are read into the config"""
    cfg = Config()
    os.environ["ROBOTMK_foo_bar1"] = "1"
    os.environ["ROBOTMK_foo_bar2"] = "2"
    os.environ["ROBOTMK_foo_bar3"] = "3"
    os.environ["DONT_foo_bar4"] = "4"
    cfg.read_cfg_vars(path=None)
    assert str(cfg.configdict["foo"]["bar1"]) == "1"
    assert str(cfg.configdict["foo"]["bar2"]) == "2"
    assert str(cfg.configdict["foo"]["bar3"]) == "3"
    # assert not
    assert not "bar4" in cfg.configdict["foo"]


def test_dotcfg2env():
    """Tests if the setenv() function works"""
    environ = {}
    dotstrings = {
        "common.logdir": "/another/path",
        "suitecfg.uuid": "1234",
        "suitecfg.run.rcc": False,
    }
    cfg = Config()
    cfg.set("common.suiteuname", "foo_suite")
    cfg.dotcfg_to_env(dotstrings, environ=environ)
    assert environ["ROBOTMK_common_logdir"] == "/another/path"
    assert environ["ROBOTMK_suites_foo__suite_uuid"] == "1234"
    assert environ["ROBOTMK_suites_foo__suite_run_rcc"] == "false"


def test_cfg2env():
    """Tests the conversion of a config dictionary to environment variables."""
    environ = {}
    cfg = Config()
    cfg.set("common.log_level", "INFO")
    cfg.set("common.suiteuname", "foo_suite")
    cfg.set("suitecfg.my_value", "qux")
    cfg.set("suitecfg.run.rcc", False)
    cfg.set("suitecfg.my_list", ["one", "two", "three"])
    cfg.set("suitecfg.my_dict", {"foo": "one", "bar": "two", "baz": "three"})
    cfg.set(
        "suitecfg.my_list_of_dict",
        [
            {"foo": "one", "bar": "two", "baz": "three"},
            {"foo": "one", "bar": "two", "baz": "three"},
            {"foo": "one", "bar": "two", "baz": "three"},
        ],
    )
    cfg.cfg_to_environment(cfg.configdict, environ=environ)
    # simple values
    assert environ["ROBOTMK_common_log__level"] == "INFO"
    assert environ["ROBOTMK_common_suiteuname"] == "foo_suite"
    assert environ["ROBOTMK_suites_foo__suite_my__value"] == "qux"
    assert environ["ROBOTMK_suites_foo__suite_run_rcc"] == "false"
    # lists
    assert environ["ROBOTMK_suites_foo__suite_my__list_0"] == "one"
    assert environ["ROBOTMK_suites_foo__suite_my__list_1"] == "two"
    assert environ["ROBOTMK_suites_foo__suite_my__list_2"] == "three"
    # dicts
    assert environ["ROBOTMK_suites_foo__suite_my__dict_foo"] == "one"
    assert environ["ROBOTMK_suites_foo__suite_my__dict_bar"] == "two"
    assert environ["ROBOTMK_suites_foo__suite_my__dict_baz"] == "three"
    # list of dicts
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_foo"] == "one"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_bar"] == "two"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_0_baz"] == "three"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_foo"] == "one"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_bar"] == "two"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_1_baz"] == "three"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_foo"] == "one"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_bar"] == "two"
    assert environ["ROBOTMK_suites_foo__suite_my__list__of__dict_2_baz"] == "three"


# TODO: split_varstring
# TODO: config validation
