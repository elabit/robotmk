#   1) List of dicts for DSL 0,1,2...
#       2) inventory_suites: list of Suite names the inventory function should find
#           3) check_item: The name of the item to be checked by the check (see Argument #4 in 
#              dict 'check_test_params' in front of the check test function
#               4) checkgroup_parameters file in test/fixtures/checkgroup_parameters (without .py extension), 
#                  can containing anything which can be set in the check's WATO page
#                   5) svc_status: The expected Nagios state of the suite
#                   5) svc_output: A Regex which is expected to match the Output 
#                   5) perfdata: A list of tuples like this. To get the tuple, set 
#                       a breakpoint at iqLA3EOq and debug the content of result[2]
[
    # discovery_suite_level 0
    {
        'inventory_suites': ['Archivetool'],
        'check_suites' : {
            'Archivetool': {
                # checkgroup_parameters file
                'perfdata_all_tests': {
                    'svc_status': 0,
                    'svc_output': ".*'Archivetool': PASS.*, OK:.*'Archivetool': PASS.*'Archivetool': PASS.*",
                },
            }
        },
    },
    # discovery_suite_level 1
    {},
    # discovery_suite_level 2
    {
        'inventory_suites': ['DUMMY', 'ARCHIVETOOL Suche LKR AI', 'ARCHIVETOOL Suche LKR FR', 'ARCHIVETOOL Suche LKR OW', 'ARCHIVETOOL Suche LKR SG', 'ARCHIVETOOL Suche LKR SH', 'ARCHIVETOOL Suche LKR UR', 'ARCHIVETOOL Suche LKR ZH'],
        'check_suites' : {
            'ARCHIVETOOL Suche LKR AI': {
                # checkgroup_parameters file
                'perfdata_all_tests': {
                    'svc_status': 0,
                    'svc_output': ".*ARCHIVETOOL Suche LKR AI': PASS .*OK:.*'ARCHIVETOOL Suche LKR AI': PASS.*'ARCHIVETOOL Suche': PASS.*",
                    'perfdata'  : [
                        ('s1-s1-t2_ARCHIVETOOL_Suche_LKR_AI', '40.08')
                    ]
                },
            }
        },
    },
]