from pytest_check_mk import OK, WARNING, CRITICAL, UNKNOWN

test_for = 'foobar'


sample_plugin_output = '''
<<<foobar>>>
FOO BAR
'''

def test_check_info(checks):
    info = checks['foobar'].check_info()
    assert info['service_description'] == "FOOBAR"

def test_inventory(checks):
    assert checks['foobar'].inventory(sample_plugin_output) == []


def test_check(checks):
    item = None
    params = None
    assert checks['foobar'].check(item, params, sample_plugin_output) == (UNKNOWN, 'UNKNOWN - Check not implemented')


def test_settings(checks):
    assert checks['foobar'].service_description == 'FOOBAR'
    assert not checks['foobar'].has_perfdata