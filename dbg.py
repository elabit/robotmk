#!/usr/bin/python
# Before use:
# - add "check_info = {}"



import ipdb
import robotmk as rf


xml = '/omd/sites/cmk/multisuites_wo_header'
file = open(xml, "r")
content = file.readlines()
parsed = rf.parse_robot(content)

ipdb.set_trace(context=5)
#rf.inventory_robot(parsed)
state, msg, perfdata = rf.check_robot("Mkdemo", [], parsed)
print "Debugger ended."

