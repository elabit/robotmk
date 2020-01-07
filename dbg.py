#!/usr/bin/python
# Before use:
# - add "check_info = {}"



import ipdb
import robotmk as rf


xml = '/omd/sites/cmk/multisuites_wo_header'
file = open(xml, "r")
content = file.readlines()



xmlname = rf.parse_robot(content)

result = rf.ExecutionResult(xmlname)
suite_metrics = rf.SuiteMetrics(2)
ipdb.set_trace(context=5)
result.visit(suite_metrics)

#[ s.name for s in suite_metrics.data]

ipdb.set_trace(context=5)
rf.inventory_robot(xmlname)

print "Debugger ended."

