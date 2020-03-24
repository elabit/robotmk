[
    # discovery_suite_level 0
    {
        'inventory_suites': ['1S 3T'],
        'check_suites' : {
            '1S 3T': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': 'Suite 1S 3T: PASS'
                },
                # Check that Keyword MySleepSleep gets not recursed
                'MySleepSleep_0': {
                    'svc_status': 0,
                    'svc_output': '.*Test Test4 - 3 Nested Sleeps: PASS.*?Keyword MySleepSleep: PASS \(\d* s\)$'
                },
                # Check that Keyword MySleepSleep gets recursed only 1 level deep 
                'MySleepSleep_1': {
                    'svc_status': 0,
                    'svc_output': '.*Test Test4 - 3 Nested Sleeps: PASS.*?Keyword MySleepSleep: PASS \(\d* s\).*?Keyword MySleep: PASS \(\d* s\)$'
                },
            }
        },
    },
]