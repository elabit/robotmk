#!.pyenv/plugin/bin/python
import os
import pytest
import robotmk
from contextlib import redirect_stdout
import io
import ast
import xml.etree.ElementTree as ET
import xml.etree.ElementTree as ET

plugin_test_params = [
    # param: Test folder below test/fixtures/plugin
    '1_execute_all_suites',
]
@pytest.mark.parametrize("test_dir", plugin_test_params)
def test_agent_plugin(test_dir):
    test_path = "./test/fixtures/plugin/%s" % test_dir
    os.environ["AGENT_CFG_DIR"] = test_path
    # os.environ["ROBOTMK_CFG_FILE"] = "robotmk.yml"

    agent_output = robot_start()
    all_xml = agent_output.split('<<<robotmk:sep(0)>>>\n')[1:]
    allsuites = []
    for xml in all_xml:
        oxml = ET.fromstring(xml)
        suite = oxml.find('suite')
        allsuites.append(suite.attrib['name'])
    expected_data = read_expected_data(test_path + '/expected.py')
    assert allsuites == expected_data['suites']

def test_agent_plugin_arg_tags_include():
    test_path = "./test/fixtures/plugin/%s" % '3_arguments_tags_include'
    os.environ["AGENT_CFG_DIR"] = test_path

    agent_output = robot_start()
    all_xml = agent_output.split('<<<robotmk:sep(0)>>>\n')[1:]
    alltests = []
    for xml in all_xml:
        oxml = ET.fromstring(xml)
        tests = oxml.findall('.//test')
        alltests.extend([test.attrib['name'] for test in tests])
    expected_data = read_expected_data(test_path + '/expected.py')
    assert alltests == expected_data['test_names']

def test_agent_plugin_arg_tags_exclude():
    test_path = "./test/fixtures/plugin/%s" % '4_arguments_tags_exclude'
    os.environ["AGENT_CFG_DIR"] = test_path

    agent_output = robot_start()
    all_xml = agent_output.split('<<<robotmk:sep(0)>>>\n')[1:]
    alltests = []
    for xml in all_xml:
        oxml = ET.fromstring(xml)
        tests = oxml.findall('.//test')
        alltests.extend([test.attrib['name'] for test in tests])
    expected_data = read_expected_data(test_path + '/expected.py')
    assert [alltests[0],alltests[-1]] == expected_data['test_names']

def test_agent_plugin_arg_vars():
    test_path = "./test/fixtures/plugin/2_arguments_variables"
    expected_data = read_expected_data(test_path + '/expected.py')
    os.environ["AGENT_CFG_DIR"] = test_path

    agent_output = robot_start()
    xml = agent_output.split('<<<robotmk:sep(0)>>>\n')[1]
    oxml = ET.fromstring(xml)
    message_values = [ msg.text for msg in oxml.findall('.//msg') ]
    for value in expected_data['var_values']:
        assert value in message_values

def test_agent_plugin_arg_name():
    test_path = "./test/fixtures/plugin/5_arguments_name"
    expected_data = read_expected_data(test_path + '/expected.py')
    os.environ["AGENT_CFG_DIR"] = test_path

    agent_output = robot_start()
    xml = agent_output.split('<<<robotmk:sep(0)>>>\n')[1]
    oxml = ET.fromstring(xml)
    suitename = oxml.find('./suite') 
    assert suitename.attrib['name'] == expected_data['suite_name']

def robot_start():
    # capture stdout of the plugin execution
    f = io.StringIO()
    with redirect_stdout(f):
        robotmk.start()
    return f.getvalue()

#   _          _
#  | |        | |
#  | |__   ___| |_ __   ___ _ __
#  | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | | |  __/ | |_) |  __/ |
#  |_| |_|\___|_| .__/ \___|_|
#               | |
#               |_|


def read_expected_data(file):
    data = ast.literal_eval(open(file, 'r').read())
    return data



if __name__ == '__main__':
    test_agent_plugin()
