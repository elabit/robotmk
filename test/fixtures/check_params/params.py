# {
#   'output_depth': {
#     'output_depth_suites': [('^A.*', 2), ('^B.*', 1)],
#     'output_depth_keywords': [('^Built.*', 2)]
#   },
#   'runtime_threshold': {
#     'runtime_threshold_suites': [(
#       '.*A.*', 10), ('.*B.*', 20), ('.*C.*', 30)],
#     'runtime_threshold_keywords': [('Built.*', 22)]
#   },
#   'perfdata_creation': {
#     # 'perfdata_creation_suites': ['Mkdemo']
#     'perfdata_creation_tests': ['test-A-.*']
#   }
# }

####
{
  'output_depth': {
    'output_depth_suites': [('^A.*', 2), ('^B.*', 1)],
    'output_depth_keywords': [('^Built.*', 2)]
  },
  'runtime_threshold': {
    'runtime_threshold_suites': [(
      '.*A.*', 10), ('.*B.*', 20), ('.*C.*', 30)],
    'runtime_threshold_keywords': [('Built.*', 22)]
  },
  # 'perfdata_creation': {
  #   # 'perfdata_creation_suites': ['Mkdemo']
  #   # 'perfdata_creation_tests': ['test-A-.*']
  # }
}