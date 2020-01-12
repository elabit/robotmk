import pytest_check_mk
#from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN
import ipdb
ipdb.set_trace(context=5)
test_for = 'robotmk'


sample_plugin_output = '''
<<<foobar>>>
FOO BAR
'''


def test_inventory(checks):
    assert checks['foobar'].inventory(sample_plugin_output) == []


def test_check(checks):
    item = None
    params = None
    assert checks['foobar'].check(item, params, sample_plugin_output) == (UNKNOWN, 'UNKNOWN - Check not implemented')


def test_settings(checks):
    assert checks['foobar'].service_description == 'FOOBAR'
    assert not checks['foobar'].has_perfdata
