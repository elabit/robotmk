{
  'perfdata_creation': {
    # 'perfdata_creation_suites': ['.*'],
    'perfdata_creation_tests': ['Test4.*'],
    'perfdata_creation_keywords': ['Compare.*'],
  },
  'runtime_threshold': {
    # 'runtime_threshold_suites': [('.*', 1, 2)],
    'runtime_threshold_tests': [('.*Test4.*', 0.5, 3)],
    'runtime_threshold_keywords': [('Compare Numbers with custom msg', 0.8, 2.1)],
  },
  'includedate' : 'yes'  
}