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
                    'svc_output': ".*?\[S\] 'Subsuite3': PASS \(\d+\.\d+s\), WARNING: Suite Sub3 suite1 over runtime, Suite Sub3 suite2 over runtime.*?\[S\] 'Sub3 suite1': PASS \(\d+\.\d+s, WARNING: > \d+\.\d+s\).*?\[S\] 'Sub3 suite2': PASS \(\d+\.\d+s, WARNING: > \d+\.\d+s\)",
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
            }
        },
    },
]