#   1) List of dicts for DSL 0,1,2...
#       2) inventory_items: list of Suite names the inventory function should find
#           3) check_suites: The name of the item to be checked by the check (see Argument #4 in 
#              dict 'check_test_params' in front of the check test function
#               4) checkgroup_parameters file in test/fixtures/checkgroup_parameters (without .py extension), 
#                  can containing anything which can be set in the check's WATO page
#                   5) svc_status: The expected Nagios state of the suite
#                   5) svc_output: A Regex which is expected to match the Output  

[
    # discovery_suite_level 0
    {
        'inventory_items': ['1 Alltest'],
        'check_suites' : {
            '1 Alltest': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 2,
                    'svc_output': ".*FAIL.*",
                },
                '1S_3T_all': {
                    'svc_status': 2,
                    'svc_output': ".*'1 Alltest': FAIL.*checks:.*",
                    'perfdata'  : [
                        ('s1-t1_Test4_sleep_and_custom_test_msg', '\d+\.\d+'),
                        ('s1-t2-k1_Compare_Numbers_with_default_msg', '\d+\.\d+'),
                        ('s1-t3-k1_Compare_Numbers_with_custom_msg', '\d+\.\d+'),
                        ('s1-t4-k1-k1_Compare_Numbers_with_custom_msg', '\d+\.\d+'),
                    ]
                },
            }
        },
    },
]