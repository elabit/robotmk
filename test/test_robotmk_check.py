from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import os
import pytest
import ast
import re

test_for = 'robotmk'

#   _            _       
#  | |          | |      
#  | |_ ___  ___| |_ ___ 
#  | __/ _ \/ __| __/ __|
#  | ||  __/\__ \ |_\__ \
#   \__\___||___/\__|___/
                       
# Info test function                       
def test_check_info(checks):
    info = checks['robotmk'].check_info
    assert info['service_description'] == "Robot"
    assert info['group'] == "robotmk"


# Inventory test function
inventory_test_params = [
    ('1S_3T', 0),
    ('1S_3S_2S_3T', 0),
    ('1S_3S_2S_3T', 1),
    ('1S_3S_2S_3T', 2),
]
@pytest.mark.parametrize("testsuite, discovery_suite_level", inventory_test_params)
def test_inventory_mk(checks, monkeypatch, testsuite, discovery_suite_level):
    mk_output = read_mk_input(testsuite + '/input_check.json')
    expected_data = read_expected_data(testsuite + '/expected.py')[discovery_suite_level]
    patch(checks.module, monkeypatch, 'discovery_suite_level_%d.py' % discovery_suite_level)
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert_inventory(inventory, expected_data['inventory_suites'])

# Check test function
check_test_params = [
    # 1 Test Suite folder
    # 2 discovery suite level
    # 3 check item
    # 4 checkgroup_parameters file name (without .py extension)
    # 1       2   3       4
    ('1S_3T', 0, '1S 3T', None),
    ('1S_3T', 0, '1S 3T', 'MySleepSleep_0'),
    ('1S_3T', 0, '1S 3T', 'MySleepSleep_1'),
    ('1S_3T', 0, '1S 3T', 'MySleep_perfdata'),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', None),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', 'Subsuite1_0'),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', 'Subsuite1_1'),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', 'Subsuites_perfdata'),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', 'Tests_perfdata'),
    ('1S_3S_2S_3T', 0, '1S 3S 2S 3T', 'runtime_test_2sec'),
    ('1S_3S_2S_3T', 1, 'Subsuite1', None),
    ('1S_3S_2S_3T', 1, 'Subsuite3', 'Suite_Sub3_suites_2seconds'),
    ('1S_3S_2S_3T', 2, 'Sub1 suite1', None),
]
@pytest.mark.parametrize("testsuite, discovery_suite_level, item, checkgroup_parameters", check_test_params)
def test_check_mk(checks, monkeypatch, testsuite, discovery_suite_level, item, checkgroup_parameters):
    mk_output = read_mk_input('%s/input_check.json' % testsuite)
    expected_data = read_expected_data(testsuite + '/expected.py')[discovery_suite_level]['check_suites'][item][checkgroup_parameters]
    patch(checks.module, monkeypatch, 'discovery_suite_level_%d.py' % discovery_suite_level)
    params = read_mk_checkgroup_params(checkgroup_parameters)

    result = checks['robotmk'].check_mk(item, params, mk_output) 
    assert result[0]== expected_data['svc_status']
    expected_output = expected_data['svc_output']
    assert re.match(expected_output, result[1], re.DOTALL)
    if 'perfdata' in expected_data and expected_data['perfdata']: 
        expected_perfdata_list = expected_data['perfdata']
        expected_perflabels = [ x[0] for x in expected_perfdata_list]
        for perfdata in result[2]:
            assert perfdata[0] in expected_perflabels
            expected_index = expected_perflabels.index(perfdata[0])
            expected_perfdata = expected_perfdata_list[expected_index]
            # Value
            assert re.match(expected_perfdata[1], perfdata[1])
            # Warning
            if len(expected_perfdata) == 3: 
                assert re.match(expected_perfdata[2], perfdata[2])


    # assert result[1].startswith(expected_output)


#   _          _                 
#  | |        | |                
#  | |__   ___| |_ __   ___ _ __ 
#  | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | | |  __/ | |_) |  __/ |   
#  |_| |_|\___|_| .__/ \___|_|   
#               | |              
#               |_|              

def read_mk_input(file):
    datafile = 'test/fixtures/robot/' + file
    return eval(open(datafile, 'r').read())

def read_mk_checkgroup_params(file):
    if file: 
        paramfile = 'test/fixtures/checkgroup_parameters/%s.py' % file
        try: 
            params = eval(open(paramfile, 'r').read())
        except: 
            params = None
        return params
    else: 
        return None

def read_mk_inventory_rules(file):
    rulefile = 'test/fixtures/check_params/' + file
    data = eval(open('test/fixtures/inventory_robotmk_rules/%s' % rulefile).read())

def read_expected_data(file):
    datafile = 'test/fixtures/robot/' + file
    data = ast.literal_eval(open(datafile, 'r').read())
    return data

def patch(module, monkeypatch, rulefile):
    data = eval(open('test/fixtures/inventory_robotmk_rules/%s' % rulefile).read())

    # patch the data
    monkeypatch.setattr(module, "inventory_robotmk_rules", data)
    # define the function to patch
    def mock_host_extra_conf_merged(hostname, inventory_robotmk_rules):
        return module.inventory_robotmk_rules[0]['value'] 
    # patch the function
    monkeypatch.setattr(module, "host_extra_conf_merged", mock_host_extra_conf_merged)

# Checks an inventory object for suites
def assert_inventory(inventory, suites):
    assert hasattr(inventory, '__iter__')
    _suites = map(lambda x: x[0], inventory)
    assert suites == _suites


