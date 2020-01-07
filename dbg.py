#!/usr/bin/python
# Before use:
# - add "check_info = {}"



import ipdb
import robotmk as rf


ipdb.set_trace(context=5)
xml = '/omd/sites/cmk/multisuites_wo_header'
file = open(xml, "r")
content = file.readlines()



xmlname = rf.parse_robot(content)

result = rf.ExecutionResult(xmlname)
suite_metrics = rf.SuiteMetrics(5)
result.visit(suite_metrics)


ipdb.set_trace(context=5)
rf.inventory_robot(xmlname)

print "Debugger ended."

