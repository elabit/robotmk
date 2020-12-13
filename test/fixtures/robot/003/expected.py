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
                    'svc_output': ".*'Testsuite': PASS.*\[T\] 'TestCase 1 CheckWrappedKeyword': PASS.*\[K\] 'Run Keyword And Return Status': PASS \(\$\{passed\} = False\).*\[K\] 'Keyword With A False Assertion': FAIL.*\[K\] 'Should Be Equal': FAIL \(This is a custom message for kw exception.: 1 != 2\).*\[T\] 'TestCase 2 CustomTestMessage': PASS \(This is a custom test message.\).*\[K\] 'Should Be Equal': PASS.*\[K\] 'Set Test Message': PASS \(Set test message to:.*This is a custom test message.\)",
                },
                '001-output_depth_kw0': {
                    'svc_status': 0,
                    'svc_output': ".*'MyFooKeyword': PASS$",
                },
                '002-output_depth_kw1': {
                    'svc_status': 0,
                    'svc_output': ".*'MyBarKeyword': PASS$",
                },
                '003-output_depth_kw2': {
                    'svc_status': 0,
                    'svc_output': ".*'MyBazKeyword': PASS$",
                },
            }
        },
    },
]