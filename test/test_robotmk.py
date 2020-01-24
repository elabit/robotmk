from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import os

test_for = 'robotmk'

datafile = 'test/fixtures/mk_output/outputagent.json'
mk_output = eval(open(datafile, 'r').read())

paramfile = 'test/fixtures/check_params/params.py'
params = eval(open(paramfile, 'r').read())

mock_inventory_robotmk_rules = eval(open('test/fixtures/inventory_rules/ruleset1.py').read())

def test_check_info(checks):
    info = checks['robotmk'].check_info
    assert info['service_description'] == "Robot"
    assert info['group'] == "robotmk"

def test_inventory_mk(checks):
    inventory = checks['robotmk'].inventory_mk(mk_output)
    assert hasattr(inventory, '__iter__') and not hasattr(inventory, '__len__')

def test_check_mk(checks, monkeypatch):
    monkeypatch.setattr(checks.module, "inventory_robotmk_rules", mock_inventory_robotmk_rules)
    def mock_host_extra_conf_merged(hostname, inventory_robotmk_rules):
        return checks.module.inventory_robotmk_rules[0]['value']
    
    monkeypatch.setattr(checks.module, "host_extra_conf_merged", mock_host_extra_conf_merged)
    item = 'Mkdemo'
    result = checks['robotmk'].check_mk(item, params, mk_output) 
    # assert result[0:2]== (OK, "foo")
    assert result[0]== (OK)


# def test_settings(checks):
#     assert checks['robotmk'].service_description == 'robotmk'
#     assert not checks['robotmk'].has_perfdata
