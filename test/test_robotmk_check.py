from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import os

test_for = 'robotmk'

paramfile = 'test/fixtures/check_params/params.py'
try: 
    params = eval(open(paramfile, 'r').read())
except: 
    params = None

def read_mk_input(file):
    datafile = 'test/fixtures/robot/' + file
    return eval(open(datafile, 'r').read())

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

def test_check_info(checks):
    info = checks['robotmk'].check_info
    assert info['service_description'] == "Robot"
    assert info['group'] == "robotmk"


def test_inventory_mk_1_1S_3T__0(checks, monkeypatch):
    mk_output = read_mk_input('1_1S_3T/output.json')
    patch(checks.module, monkeypatch, 'discovery_suite_level_0.py')
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert_inventory(inventory, ['1 Simpletest'])

def test_check_mk(checks, monkeypatch):
    mk_output = read_mk_input('1_1S_3T/output.json')
    patch(checks.module, monkeypatch, 'discovery_suite_level_0.py')

    item = '1 Simpletest'
    result = checks['robotmk'].check_mk(item, params, mk_output) 
    assert result[0]== (OK)
    assert result[1].startswith(' Suite 1 Simpletest: PASS')


def test_inventory_mk_2_1S_3S_2S_3T__0(checks, monkeypatch):
    mk_output = read_mk_input('2_1S_3S_2S_3T/output.json')
    patch(checks.module, monkeypatch, 'discovery_suite_level_0.py')
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert_inventory(inventory, ['2 1S 3S 2S 3T'])

def test_inventory_mk_2_1S_3S_2S_3T__1(checks, monkeypatch):
    mk_output = read_mk_input('2_1S_3S_2S_3T/output.json')
    patch(checks.module, monkeypatch, 'discovery_suite_level_1.py')
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert_inventory(inventory, ['Subsuite1', 'Subsuite2', 'Subsuite3'])

