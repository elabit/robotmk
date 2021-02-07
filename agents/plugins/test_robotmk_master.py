#!.pyenv/plugin/bin/python
import pytest
import os
from unittest import mock
from robotmk import RMKConfig, RMKctrl
import mergedeep

@mock.patch.dict(os.environ, {
    # test setting variables
    "ROBOTMK_SUITES_SELENIUM_TEST_ONE_VARIABLE_AAA": "111",
    "ROBOTMK_SUITES_SELENIUM_TEST_ONE_VARIABLE_BBB": "222",
    # test if preserved word "log_rotation" is kept as a suite name
    "ROBOTMK_SUITES_LOG_ROTATION_VARIABLE_CCC": "333",
    })
def test_cfg_read_from_env():
    env_dict = RMKConfig.read_env2dictionary()
    assert env_dict['suites']['selenium_test_one']['variable']['aaa'] == '111'
    assert env_dict['suites']['selenium_test_one']['variable']['bbb'] == '222'
    assert env_dict['suites']['log_rotation']['variable']['ccc'] == '333'
    conf = RMKConfig(RMKctrl)