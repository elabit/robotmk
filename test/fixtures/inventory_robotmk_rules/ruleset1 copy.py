[
    {'condition': {}, 'value': {
        'discovery_suite_level': '0',
        'check_depth': {
            'check_depth_suites': {
                'Mkdemo' : 1,
                'A-Suite1' : 1
            },
            'check_depth_keywords': {
                'Builtin.Sleep' : 1,
                'Foobar' : 2
            },
        },
        'runtime_thresholds': {
            'runtime_thresholds_suites': {
                '^Mk.*': 2,
                'A-Suite1' : 1
            },
            'runtime_thresholds_tests': {
                'test.*' : 1,
            },                                
            'runtime_thresholds_keywords': {
                'Builtin.*' : 1,
                'Foob*' : 2
            },                
        },        
        # 'perfdata_for': [ '^Mkde.*' ],
        'perfdata_for': [ '^A-Tes.*', '^C-sui.*' ],
        # 'perfdata_for': [ '^Mkde.*', 'A-su.*' ],
        'suite_runtime_thresholds' : {
            'Mkdemo'  : 60,
            '^.-suite.$'  : 120
        },
        'test_runtime_thresholds' : {
            '^test-.*'  : 60,
        },
        'keyword_runtime_thresholds' : {
            '^BuiltIn.*'  : 60,
        },
    }}
]