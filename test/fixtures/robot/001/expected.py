#   1) List of dicts for DSL 0,1,2...
#       2) inventory_items: list of Suite names the inventory function should find
#           3) items: The name of the item to be checked by the check (see Argument #4 in 
#              dict 'check_test_params' in front of the check test function
#               4) checkgroup_parameters file in test/fixtures/checkgroup_parameters (without .py extension), 
#                  can containing anything which can be set in the check's WATO page
#                   5) svc_status: The expected Nagios state of the suite
#                   5) svc_output: A Regex which is expected to match the Output  

[
    # discovery_suite_level 0
    {
        'inventory_items': ['Testsuite'],
        'items' : {
            'Testsuite': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': ".*'Testsuite': PASS.*'Testcase 1': PASS.*'Sleep': PASS \(Slept 1 second\)",
                },
                '001-thresholds_test_warn': {
                    'svc_status': 1,
                    'svc_output': ".*'Testsuite': PASS, \[T\] 'Testcase 1': \(!\) runtime=1.00s >= 0.50s.*'Testcase 1': PASS, \(!\) runtime=1.00s >= 0.50s",
                },
                '002-thresholds_test_crit': {
                    'svc_status': 2,
                    'svc_output': ".*'Testsuite': PASS, \[T\] 'Testcase 1': \(!!\) runtime=1.00s >= 0.80s.*'Testcase 1': PASS, \(!!\) runtime=1.00s >= 0.80s",
                },
                '003-thresholds_kw_warn': {
                    'svc_status': 1,
                    'svc_output': ".*'Testsuite': PASS, \[T\] 'Testcase 1': \[K\] 'Sleep': \(!\) runtime=1.00s >= 0.50s.*'Testcase 1': PASS, \[K\] 'Sleep': \(!\) runtime=1.00s >= 0.50s.*"
                },
                '004-thresholds_kw_crit': {
                    'svc_status': 2,
                    'svc_output': ".*'Testsuite': PASS, \[T\] 'Testcase 1': \[K\] 'Sleep': \(!!\) runtime=1.00s >= 0.80s.*'Testcase 1': PASS, \[K\] 'Sleep': \(!!\) runtime=1.00s >= 0.80s.*"
                },
                '005-thresholds_suite_warn': {
                    'svc_status': 1,
                    'svc_output': ".*'Testsuite': PASS, \(!\) runtime=1.03s >= 0.50s.*",
                },
                '006-thresholds_suite_crit': {
                    'svc_status': 2,
                    'svc_output': ".*'Testsuite': PASS, \(!!\) runtime=1.03s >= 0.80s.*",
                },
                '007-thresholds_perfdata_all': {
                    'svc_status': 2,
                    'svc_output': ".*'Testsuite': PASS, \(!!\) runtime=1.03s >= 0.80s, \[T\] 'Testcase 1': \(!!\) runtime=1.00s >= 0.80s, \[K\] 'Sleep': \(!!\) runtime=1.00s >= 0.80s.*",
                    'perfdata'  : [
                        ('s1_Testsuite', '1.03', '0.50', '0.80'), 
                        ('s1-t1_Testcase_1', '1.00', '0.50', '0.80'), 
                        ('s1-t1-k1_Sleep', '1.00', '0.50', '0.80'),
                    ]
                },
                '008-includedate': {
                    'svc_status': 0,
                    'svc_output': ".*'Testsuite': PASS \(last execution: 12\/13 22:06:39\).*",
                },
            }
        },
    },
    # discovery_suite_level 1
    {
        'inventory_items': ['Testcase 1'],
        'items' : {
            'Testcase 1': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': ".*(?!'Testsuite': PASS).*'Testcase 1': PASS.*'Sleep': PASS \(Slept 1 second\)",
                },
            }
        },
    },
    # discovery_suite_level 2
    {
        'inventory_items': ['Sleep'],
        'items' : {
            'Sleep': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': ".*(?!'Testsuite': PASS).*(?!'Testcase 1': PASS).*'Sleep': PASS \(Slept 1 second\)",
                },
            }
        },
    },
]