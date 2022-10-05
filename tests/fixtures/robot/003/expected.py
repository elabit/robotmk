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
                    'svc_status': 2,
                    'svc_output': "--xS-- 'Testsuite': FAIL --CRIT--\\n--xT-- 'TestCase 1 Unwrapped False Assertion': FAIL --CRIT-- \(This assertion failed.: 1 != 2\)\\n--K-- 'KeywordWithFalseAssertion': FAIL\\n--K-- 'Should Be Equal': FAIL \(This assertion failed.: 1 != 2\)\\n--T-- 'TestCase 2 Wrapped False Assertion': PASS\\n--K-- 'Run Keyword And Return Status': PASS \(passed = False\)\\n--K-- 'KeywordWithFalseAssertion': FAIL\\n--K-- 'Should Be Equal': FAIL \(This assertion failed.: 1 != 2\)\\n--xT-- 'TestCase 3 UnWrapped Fail': FAIL --CRIT-- \(This is the message of a thrown Fail.\)\\n--K-- 'FailWithMessage': FAIL\\n--K-- 'Fail': FAIL \(This is the message of a thrown Fail.\)\\n--T-- 'TestCase 4 Wrapped Fail': PASS\\n--K-- 'Run Keyword And Return Status': PASS \(passed = False\)\\n--K-- 'FailWithMessage': FAIL\\n--K-- 'Fail': FAIL \(This is the message of a thrown Fail.\)\\n--T-- 'TestCase 5 WithTestMessage': PASS \(This is a custom test message.\)\\n--K-- 'Set Test Message': PASS.*",
                },
            }
        },
    },
]