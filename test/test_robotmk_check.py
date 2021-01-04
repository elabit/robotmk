#!.tox/check/bin/python

from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import os
import sys
import pytest
import ast
import re
import codecs

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
    ('001',         'dl_0', 0),
    ('001',         'dl_1', 1),
    ('001',         'dl_2', 2),
]

    # mk_check_input = read_mk_input('%s/input_check.json' % testsuite)
    # discovery_rules = read_mk_inventory_rules(testsuite, inventory_rules)
    # expected_data = read_expected_data(testsuite, discovery_level, item, checkgroup_parameters)
    
    # patch(checks.module, monkeypatch, discovery_rules)
    # params = read_mk_checkgroup_params(testsuite, checkgroup_parameters)

    # result = checks['robotmk'].check_mk(item, params, mk_check_input) 
    # expected_output = expected_data['svc_output']

@pytest.mark.parametrize("testsuite, inventory_rules, discovery_level", inventory_test_params)
def test_inventory_mk(checks, monkeypatch, testsuite, inventory_rules, discovery_level):
    mk_check_input = read_mk_input(testsuite)
    discovery_rules = read_mk_inventory_rules(testsuite, inventory_rules)
    expected_data = read_expected_data(testsuite, discovery_level)
    patch(checks.module, monkeypatch, discovery_rules)
    inventory = checks['robotmk'].inventory_mk(mk_check_input)
    assert_inventory(inventory, expected_data['inventory_items'])

# # Multiple Suite Inventory test function
# def test_multi_inventory_mk(checks, monkeypatch):
#     # first suite
#     mk_check_input = read_mk_input('002')
#     # second suite
#     mk_check_input.extend(read_mk_input('003'))
#     discovery_rules = read_mk_inventory_rules('001', inven)
#     patch(checks.module, monkeypatch, discovery_rules)
#     inventory = checks['robotmk'].inventory_mk(mk_check_input)
#     assert_inventory(inventory, ['1S 3T', '1S 3S 2S 3T'])


# Check test function
check_test_params = [
    # 1 Test Suite folder
    # 2 inventory_rule filename (without .py extension)
    # 3 discovery_level
    # 4 check item - that should be checked (see the item comment No. 3) in the suite's "expected.py")
    # 5 checkgroup_parameters file in test/fixtures/checkgroup_parameters (without .py extension),
    #   See the item comment No. 4) in the suite's "expected.py

    # The value of 3) is the "item" = what the patterns in 2) should result in

    # 1             2       3  4              5
    # Discovery level 0,1,2 = Suite, Test, Keyword 
    ('001',         'dl_0', 0, 'Testsuite',    None),
    ('001',         'dl_1', 1, 'Testcase 1',   None),
    ('001',         'dl_2', 2, 'Sleep',        None),
    # Thresholds
    ('001',         'dl_0', 0, 'Testsuite',    '001-thresholds_test_warn'),
    ('001',         'dl_0', 0, 'Testsuite',    '002-thresholds_test_crit'),
    ('001',         'dl_0', 0, 'Testsuite',    '003-thresholds_kw_warn'),
    ('001',         'dl_0', 0, 'Testsuite',    '004-thresholds_kw_crit'),
    ('001',         'dl_0', 0, 'Testsuite',    '005-thresholds_suite_warn'),
    ('001',         'dl_0', 0, 'Testsuite',    '006-thresholds_suite_crit'),
    ('001',         'dl_0', 0, 'Testsuite',    '007-thresholds_perfdata_all'),
    ('001',         'dl_0', 0, 'Testsuite',    '008-includedate'),
    # Output depth
    ('002',         'dl_0', 0, 'Testsuite',    '001-output_depth_kw0'),
    ('002',         'dl_0', 0, 'Testsuite',    '002-output_depth_kw1'),
    ('002',         'dl_0', 0, 'Testsuite',    '003-output_depth_kw2'),
    # Check if FAILed keyword gets catched by "Run Keyword And Return Status"
    ('003',         'dl_0', 0, 'Testsuite',     None),
    # ('999_gin',     'dl_0', 0, 'E2E-Gin',       None),
    # ('999_gin',     'dl_0', 0, 'E2E-Gin',      '001-perfdata_keywords'),
    # ('999_gin',     'dl_2', 2, '2-Dossier Karte Wasser_Abfluss_BAFU',      '002-submessages'),
]
@pytest.mark.parametrize("testsuite, inventory_rules, discovery_level, item, checkgroup_parameters", check_test_params)
def test_check_mk(checks, monkeypatch, testsuite, inventory_rules, discovery_level, item, checkgroup_parameters):
    mk_check_input = read_mk_input(testsuite)
    discovery_rules = read_mk_inventory_rules(testsuite, inventory_rules)
    expected_data = read_expected_data(testsuite, discovery_level, item, checkgroup_parameters)
    
    patch(checks.module, monkeypatch, discovery_rules)
    params = read_mk_checkgroup_params(testsuite, checkgroup_parameters)

    result = checks['robotmk'].check_mk(item, params, mk_check_input) 
    expected_output = expected_data['svc_output']
    assert result[0]== expected_data['svc_status']
    assert re.match(expected_output, result[1], re.DOTALL)
    #iqLA3EOq
    if 'perfdata' in expected_data and expected_data['perfdata']: 
        expected_perfdata_list = expected_data['perfdata']
        expected_perflabels = [ x[0] for x in expected_perfdata_list]
        for perfdata in result[2]:
            assert perfdata[0] in expected_perflabels
            expected_index = expected_perflabels.index(perfdata[0])
            expected_perfdata = expected_perfdata_list[expected_index]
            # Value
            assert re.match(expected_perfdata[1], perfdata[1])
            # Check perfdata if they are expected
            if len(expected_perfdata) > 2: 
                assert perfdata[2:] == expected_perfdata[2:]

#   _          _                 
#  | |        | |                
#  | |__   ___| |_ __   ___ _ __ 
#  | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | | |  __/ | |_) |  __/ |   
#  |_| |_|\___|_| .__/ \___|_|   
#               | |              
#               |_|              

def read_mk_input(testsuite):
    datafile = "test/fixtures/robot/%s/input_check.json" % (testsuite)
    return eval(codecs.open(datafile, 'r', 'utf-8').read())

# Load the WATO check settings
def read_mk_checkgroup_params(testsuite, file):
    if file: 
        datafile = "test/fixtures/robot/%s/check_params/%s.py" % (testsuite, file)
        data = ast.literal_eval(codecs.open(datafile, 'r', 'utf-8').read())
        return data
    else: 
        return None

def read_mk_inventory_rules(testsuite, file):
    datafile = "test/fixtures/robot/%s/inventory_rules/%s.py" % (testsuite, file)
    data = ast.literal_eval(codecs.open(datafile, 'r', 'utf-8').read())
    # return eval(open('test/fixtures/inventory_robotmk_rules/%s.py' % rulefile).read())
    return data

def read_expected_data(testsuite, discovery_level, item=None, checkgroup_parameters=None):
    datafile = "test/fixtures/robot/%s/expected.py" % testsuite
    data = eval_file(datafile)
    try: 
        expected_data_dl = data[discovery_level]
    except: 
        print "ERROR: %s does not contain a valid entry for discovery level '%s'!" % (datafile, discovery_level)
        sys.exit(1)
    if item != None:
        try:
            expected_data = expected_data_dl['items'][item][checkgroup_parameters]
        except: 
            print "ERROR: %s does not contain a valid entry for either item '%s', discovery level %s and/or checkgroup_params '%s'!" % (
                datafile, item, discovery_level, checkgroup_parameters)
            sys.exit(1)
    # for inventory
    else: 
        expected_data = expected_data_dl
    return expected_data

def eval_file(datafile):
    try: 
        data = ast.literal_eval(codecs.open(datafile, 'r', 'utf-8').read())
        return data
    except: 
        print "ERROR: File %s not readable!" % (datafile)
        sys.exit(1)


def patch(module, monkeypatch, data):
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


