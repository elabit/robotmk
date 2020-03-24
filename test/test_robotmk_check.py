from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import os
import pytest
import ast

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
    mk_output = read_mk_input(testsuite + '/output.json')
    expected_data = read_expected_data(testsuite + '/expected.py')
    patch(checks.module, monkeypatch, 'discovery_suite_level_%d.py' % discovery_suite_level)
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert_inventory(inventory, expected_data[discovery_suite_level]['suites'])

# Check test function
def test_check_mk(checks, monkeypatch):
    mk_output = read_mk_input('1S_3T/output.json')
    patch(checks.module, monkeypatch, 'discovery_suite_level_0.py')
    params = read_mk_checkparams('params.py')

    item = '1 Simpletest'
    result = checks['robotmk'].check_mk(item, params, mk_output) 
    assert result[0]== (OK)
    assert result[1].startswith(' Suite 1 Simpletest: PASS')


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

def read_mk_checkparams(file):
    paramfile = 'test/fixtures/check_params/' + file
    try: 
        params = eval(open(paramfile, 'r').read())
    except: 
        params = None
    return params

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


