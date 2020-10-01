#   List of dicts for DSL 0,1,2...
#       inventory_suites: list of Suite names the inventory function should find
#           check_suites: The name of the item to be checked by the check (see Argument #4 in 
#           dict 'check_test_params' in front of the check test function
#               checkgroup_parameters file in test/fixtures/checkgroup_parameters (without .py extension), 
#               can containing anything which can be set in the check's WATO page
#                   svc_status: The expected Nagios state of the suite
#                   svc_output: A Regex which is expected to match the Output  

[
    # discovery_suite_level 0
    {
        'inventory_suites': ['1S 2T fail'],
        'check_suites' : {
            '1S 2T fail': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 2,
                    'svc_output': ".*'1S 2T fail': FAIL.*, CRITICAL: Test 'This first test fails' failed with 'Numbers are not equal: 10 != 100', Test 'This second test fails' failed with 'Numbers are not equal: 100 != 10'",
                },
            }
        },
    },
]