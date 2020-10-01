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
        'inventory_suites': ['1S 3S 2S 3T'],
        'check_suites' : {
            # Suite name
            '1S 3S 2S 3T': {
                # checkgroup_parameters file
                None: {
                    'svc_status': 0,
                    'svc_output': ".*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] 'Subsuite1': PASS.*?\[S\] 'Sub1 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS"
                },
                # Test that Subsuite1 does not get recursed (level 0)
                'Subsuite1_0': {
                    'svc_status': 0,
                    'svc_output': ".*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] 'Subsuite1': PASS.*?\[S\] 'Subsuite2': PASS.*?\[S\] 'Sub2 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS"
                },
                # Test that Subsuite1 gets recursed only one level deeper
                'Subsuite1_1': {
                    'svc_status': 0,
                    'svc_output': ".*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] '1S 3S 2S 3T': PASS.*?\[S\] 'Subsuite1': PASS.*?\[S\] 'Sub1 suite1': PASS.*?\[S\] 'Subsuite2': PASS.*?\[S\] 'Sub2 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS"
                },
                # Test that there are perfdata for Subsuites.*
                'Subsuites_perfdata': {
                    'svc_status': 0,
                    'svc_output': ".*?",
                    'perfdata'  : [
                        ('s1-s1_Subsuite1', '\d+\.\d+', '8.00', '10.00'), 
                        ('s1-s2_Subsuite2', '\d+\.\d+', '8.00', '10.00'), 
                        ('s1-s3_Subsuite3', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s3-s2_Sub3_suite2', '\d+\.\d+'),
                    ]
                },
                # Test that there are perfdata for Tests "Sleep the second time for.*""
                'Tests_perfdata': {
                    'svc_status': 0,
                    'svc_output': ".*?",
                    'perfdata'  : [
                        ('s1-s1-s1-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s1-s2-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s2-s1-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s2-s2-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s3-s1-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                        ('s1-s3-s2-t2_Sleep_the_second_time_for_0.1_sec', '\d+\.\d+', '8.00', '10.00'),
                    ]
                },  
                # Test that Tests "third time.*" are WARNING
                'runtime_test_2sec_warn': {
                    'svc_status': 1,
                    'svc_output': ".*?\[S\] '1S 3S 2S 3T': PASS \(\d+\.\d+s\), WARNING: Test 'Sleep the third time for 3 sec' over runtime.*?",
                },                              
                # Test that Tests "third time.*" are CRITICAL
                'runtime_test_2sec_crit': {
                    'svc_status': 2,
                    'svc_output': ".*?\[S\] '1S 3S 2S 3T': PASS \(\d+\.\d+s\), CRITICAL: Test 'Sleep the third time for 3 sec' over runtime.*?",
                },                              
            }
        },
    },
    # discovery_suite_level 1
    {
        'inventory_suites': ['Subsuite1', 'Subsuite2', 'Subsuite3'],
        'check_suites' : {
            'Subsuite1': {
                None: {
                    'svc_status': 0,
                    'svc_output': ".*?\[S\] 'Subsuite1': PASS.*?\[S\] 'Subsuite1': PASS.*?\[S\] 'Sub1 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS",
                },
            },
            'Subsuite3': {
                # Two suites run for 3 seconds > 2 WARN
                'Suite_Sub3_suites_2seconds': {
                    'svc_status': 1,
                    'svc_output': ".*?\[S\] 'Subsuite3': PASS \(\d+\.\d+s\), WARNING: Suite 'Sub3 suite1' over runtime, Suite 'Sub3 suite2' over runtime.*?\[S\] 'Sub3 suite1': PASS \(\d+\.\d+s, WARNING: > \d+\.\d+s\).*?\[S\] 'Sub3 suite2': PASS \(\d+\.\d+s, WARNING: > \d+\.\d+s\)",
                },
            }
        }
    },
    # discovery_suite_level 2
    {
        'inventory_suites': ['Sub1 suite1', 'Sub1 suite2', 'Sub2 suite1', 'Sub2 suite2', 'Sub3 suite1', 'Sub3 suite2'],
        'check_suites' : {
            'Sub1 suite1': {
                None: {
                    'svc_status': 0,
                    'svc_output': ".*\[S\] 'Sub1 suite1': PASS.*?\[S\] 'Sub1 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS",
                }
            },
            'Sub1 suite1': {
                'Perfdata_SDL2': {
                    'svc_status': 0,
                    'svc_output': ".*\[S\] 'Sub1 suite1': PASS.*?\[S\] 'Sub1 suite1': PASS.*?\[T\] 'Sleep the first time for 0.1 sec': PASS.*?\[K\] 'Sleep': PASS",
                }
            },
        },
    },
]