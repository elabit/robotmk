{
  'perfdata_creation': {
    'perfdata_creation_suites': ['.*'],
    'perfdata_creation_tests': ['.*assertion.*'],
    'perfdata_creation_keywords': ['Compare.*'],
  },
  'runtime_threshold': {
    'runtime_threshold_suites': [('.*', 1, 2)],
    'runtime_threshold_tests': [('.*assertion.*', 2, 3)],
    'runtime_threshold_keywords': [('.*m.*', 0.8, 2.1)],
  }  
}