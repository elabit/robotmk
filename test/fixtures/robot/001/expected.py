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
        'inventory_items': {
            'dl_0': {
                'inventory_items': ['Testsuite'],
            },
            'dl_0_prefix': {
                'inventory_items': ['TESTPREFIX Testsuite'],
            },
        },
        'items' : {
            'Testsuite': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': u"--S-- 'Testsuite': PASS",
                },
                '000-thresholds_test_ok': {
                    'svc_status': 0,
                    'svc_output': "--S-- 'Testsuite': PASS.*--T-- 'Testcase 1': PASS\\n.*--K-- 'Sleep'.*",
                },
                '000-thresholds_test_okshowallruntimes': {
                    'svc_status': 0,
                    'svc_output': "--S-- 'Testsuite': PASS.*--T-- 'Testcase 1': PASS, --RUNTIME--.* ",
                },
                '001-thresholds_test_warn': {
                    'svc_status': 1,
                    'svc_output': "--xS-- 'Testsuite': PASS --WARN--, --t-- 'Testcase 1': --WARN-- --RUNTIME-- >= 0.50s\\n--xT-- 'Testcase 1': PASS --WARN--, --WARN-- --RUNTIME-- >= 0.50s.*",
                },
                '002-thresholds_test_crit': {
                    'svc_status': 2,
                    'svc_output': "--xS-- 'Testsuite': PASS --CRIT--, --t-- 'Testcase 1': --CRIT-- --RUNTIME-- >= 0.80s\\n--xT-- 'Testcase 1': PASS --CRIT--, --CRIT-- --RUNTIME-- >= 0.80s.*",
                },
                '003-thresholds_kw_warn': {
                    'svc_status': 1,
                    'svc_output': "--xS-- 'Testsuite': PASS --WARN--\\n--xT-- 'Testcase 1': PASS --WARN--, --k-- 'Sleep': --WARN-- --RUNTIME-- >= 0.50s\\n--xK-- 'Sleep': PASS \(Slept 1 second\), --WARN-- --RUNTIME-- >= 0.50.*"
                },
                '004-thresholds_kw_crit': {
                    'svc_status': 2,
                    'svc_output': "--xS-- 'Testsuite': PASS --CRIT--\\n--xT-- 'Testcase 1': PASS --CRIT--, --k-- 'Sleep': --CRIT-- --RUNTIME-- >= 0.80s\\n--xK-- 'Sleep': PASS \(Slept 1 second\), --CRIT-- --RUNTIME-- >= 0.80.*"
                },
                '005-thresholds_suite_warn': {
                    'svc_status': 1,
                    'svc_output': "--xS-- 'Testsuite': PASS --WARN--, --WARN-- --RUNTIME-- >= 0.50s.*",
                },
                '006-thresholds_suite_crit': {
                    'svc_status': 2,
                    'svc_output': "--xS-- 'Testsuite': PASS --CRIT--, --CRIT-- --RUNTIME-- >= 0.80s.*",
                },
                '007-thresholds_perfdata_all': {
                    'svc_status': 2,
                    'svc_output': "--xS-- 'Testsuite': PASS --CRIT--, --CRIT-- --RUNTIME-- >= 0.80s, --t-- 'Testcase 1': --CRIT-- --RUNTIME-- >= 0.80s\\n--xT-- 'Testcase 1': PASS --CRIT--, --CRIT-- --RUNTIME-- >= 0.80s, --k-- 'Sleep': --CRIT-- --RUNTIME-- >= 0.80s.*",
                    'perfdata'  : [
                        ('s1_Testsuite', '1.03', '0.50', '0.80'), 
                        ('s1_t1_Testcase_1', '1.00', '0.50', '0.80'), 
                        ('s1_t1_k1_Sleep', '1.00', '0.50', '0.80'),
                    ]
                },
                '008-includedate': {
                    'svc_status': 0,
                    'svc_output': "--S-- 'Testsuite': PASS --LASTEXECUTION-- \\n--T-- 'Testcase 1': PASS --LASTEXECUTION-- .*",
                },
            }
        },
    },
    # discovery_suite_level 1
    {
        'inventory_items': {
            'dl_1': {
                'inventory_items': ['Testcase 1'],
            },
        },        
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
        'inventory_items': {
            'dl_2': {
                'inventory_items': ['Sleep'],
            },
        },          
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