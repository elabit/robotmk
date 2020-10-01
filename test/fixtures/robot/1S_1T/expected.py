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
        'inventory_suites': ['1S 3T'],
        'check_suites' : {
            '1S 3T': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': ".*'1S 3T': PASS",
                },
                # Check that Keyword MySleepSleep gets not recursed
                'MySleepSleep_0': {
                    'svc_status': 0,
                    'svc_output': ".*\[T\] 'Test4 - 3 Nested Sleeps': PASS.*?\[K\] 'MySleepSleep': PASS \(\d+\.\d+s\)$"
                },
                # Check that Keyword MySleepSleep gets recursed only 1 level deep 
                'MySleepSleep_1': {
                    'svc_status': 0,
                    'svc_output': ".*\[T\] 'Test4 - 3 Nested Sleeps': PASS.*?\[K\] 'MySleepSleep': PASS \(\d+\.\d+s\).*?\[K\] 'MySleep': PASS \(\d+\.\d+s\)$"
                },
                # Check Keyword perfdata for MySleep.*
                'MySleep_perfdata': {
                    'svc_status': 0,
                    'svc_output': ".*",
                    'perfdata'  : [
                        ('s1-s1-t2-k1_MySleep', '\d+\.\d+'),
                        ('s1-s1-t3-k1_MySleepSleep', '\d+\.\d+'),
                        ('s1-s1-t3-k1-k1_MySleep', '\d+\.\d+'),
                        ('s1-s1-t4-k1_MySleepSleepSleep', '\d+\.\d+'),
                        ('s1-s1-t4-k1-k1_MySleepSleep', '\d+\.\d+'),
                        ('s1-s1-t4-k1-k1-k1_MySleep', '\d+\.\d+'),
                    ]
                },
            }
        },
    },
]