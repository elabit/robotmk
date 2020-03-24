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
                    'svc_output': '^Suite 1S 3S 2S 3T: PASS.*?Suite 1S 3S 2S 3T: PASS.*?Suite Subsuite1: PASS.*?Suite Sub1 suite1: PASS.*?Test Sleep the first time for 1 sec: PASS.*?Keyword Sleep: PASS'
                },
                # Test that Subsuite1 does not get recursed (level 0)
                'Subsuite1_0': {
                    'svc_status': 0,
                    'svc_output': '^Suite 1S 3S 2S 3T: PASS.*?Suite 1S 3S 2S 3T: PASS.*?Suite Subsuite1: PASS.*?Suite Subsuite2: PASS.*?Suite Sub2 suite1: PASS.*?Test Sleep the first time for 1 sec: PASS.*?Keyword Sleep: PASS'
                },
                # Test that Subsuite1 gets recursed only one level deeper
                'Subsuite1_1': {
                    'svc_status': 0,
                    'svc_output': '^Suite 1S 3S 2S 3T: PASS.*?Suite 1S 3S 2S 3T: PASS.*?Suite Subsuite1: PASS.*?Suite Sub1 suite1: PASS.*?Suite Subsuite2: PASS.*?Suite Sub2 suite1: PASS.*?Test Sleep the first time for 1 sec: PASS.*?Keyword Sleep: PASS'
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
                    'svc_output': '^Suite Subsuite1: PASS.*?Suite Subsuite1: PASS.*?Suite Sub1 suite1: PASS.*?Test Sleep the first time for 1 sec: PASS.*?Keyword Sleep: PASS',
                },
            },
        }
    },
    # discovery_suite_level 2
    {
        'inventory_suites': ['Sub1 suite1', 'Sub1 suite2', 'Sub2 suite1', 'Sub2 suite2', 'Sub3 suite1', 'Sub3 suite2'],
        'check_suites' : {
            'Sub1 suite1': {
                None: {
                    'svc_status': 0,
                    'svc_output': '^Suite Sub1 suite1: PASS.*?Suite Sub1 suite1: PASS.*?Test Sleep the first time for 1 sec: PASS.*?Keyword Sleep: PASS',
                }
            }
        },
    },
]