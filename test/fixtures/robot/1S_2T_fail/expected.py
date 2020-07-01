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