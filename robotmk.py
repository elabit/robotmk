#!/usr/bin/python

# -*- encoding: utf-8; py-indent-offset: 4 -*-
# +------------------------------------------------------------------+
# |             ____ _               _        __  __ _  __           |
# |            / ___| |__   ___  ___| | __   |  \/  | |/ /           |
# |           | |   | '_ \ / _ \/ __| |/ /   | |\/| | ' /            |
# |           | |___| | | |  __/ (__|   <    | |  | | . \            |
# |            \____|_| |_|\___|\___|_|\_\___|_|  |_|_|\_\           |
# |                                                                  |
# | Copyright Mathias Kettner 2020             mk@mathias-kettner.de |
# +------------------------------------------------------------------+
#
# This file is part of Check_MK.
# The official homepage is at http://mathias-kettner.de/check_mk.
#
# check_mk is free software;  you can redistribute it and/or modify it
# under the  terms of the  GNU General Public License  as published by
# the Free Software Foundation in version 2.  check_mk is  distributed
# in the hope that it will be useful, but WITHOUT ANY WARRANTY;  with-
# out even the implied warranty of  MERCHANTABILITY  or  FITNESS FOR A
# PARTICULAR PURPOSE. See the  GNU General Public License for more de-
# tails. You should have  received  a copy of the  GNU  General Public
# License along with GNU Make; see the file  COPYING.  If  not,  write
# to the Free Software Foundation, Inc., 51 Franklin St,  Fifth Floor,
# Boston, MA 02110-1301 USA.

# Output of agent (unparsed result file content of "output.xml" from RF)
#<<<robot:sep(0)>>>
#<?xml version="1.0" encoding="UTF-8"?>
#<robot rpa="false" generated="20200103 16:14:32.944" generator="Robot 3.1.2 (Python 2.7.15+ on linux2)">
#<suite source="/home/elabit/Downloads/red/workspace/mkdemo" id="s1" name="Mkdemo">
#<suite source="/home/elabit/Downloads/red/workspace/mkdemo/A-Tests" id="s1-s1" name="A-Tests">
#<suite source="/home/elabit/Downloads/red/workspace/mkdemo/A-Tests/A-suite1.robot" id="s1-s1-s1" name="A-suite1">
#<test id="s1-s1-s1-t1" name="test-A-1-1">
#...

# 's1-s3-s2-t1-k1'

from robot.api import ExecutionResult, ResultVisitor
import tempfile
import os
from pprint import pprint


#factory_settings["robotmk_default_values"] = {
#    discovery_suite_level: 0,
#}
inventory_robot_rules = []

def parse_robot(info):
    with tempfile.NamedTemporaryFile(delete=False) as f_tmpxml:
        for line in info:
            xml_writeline(f_tmpxml, line)

    # RF result object        
    robot_result = ExecutionResult(f_tmpxml.name)
    # delete the tempfile
    os.remove(f_tmpxml.name)
    return robot_result

def inventory_robot(robot_result):
    settings = host_extra_conf_merged(host_name(), inventory_robot_rules)
    discovery_suite_level = settings.get("discovery_suite_level", 0)
    visitor = RobotMetricsVisitor(int(discovery_suite_level))
    robot_result.visit(visitor)
    for suite in visitor.data:
        # print suite.name
        yield suite.name, None

def check_robot(item, params, robot_result):
    settings = host_extra_conf_merged(host_name(), inventory_robot_rules)
    discovery_suite_level = settings.get("discovery_suite_level", 0)
    check_depth_suites = settings.get("check_depth_suites", {})
    check_depth_keywords = settings.get("check_depth_keywords", {})
    perfdata_creation_for = settings.get("perfdata_creation_for", [])
    suite_runtime_thresholds = settings.get("suite_runtime_thresholds", {})
    test_runtime_thresholds = settings.get("test_runtime_thresholds", {})
    keyword_runtime_thresholds = settings.get("keyword_runtime_thresholds", {})

    visitor = RobotMetricsVisitor(int(discovery_suite_level))
    robot_result.visit(visitor)
    for suite in visitor.data:
        if suite.name == item:
            # iteriere ab hier durch den ganzen Baum
            rc = suite.nagios_stateid
            #return rc, suite.status
            perf = suite.nagios_perfdata_recursive()
            return eval_state(suite)

# gets overwritten by __init__ when debugging
def xml_writeline(f_tmpxml, line):
    f_tmpxml.write(line[0])

# ----------------------------+--------+-------+----------+
#                             | output | rc    | perfdata |
# check_depth_suites          |   x    |       |          |
# check_depth_keywords        |   x    |       |          |
# perfdata_creation_for       |        |       |   x      |
# suite_runtime_thresholds    |   x    |  x    |          |
# test_runtime_thresholds     |   x    |  x    |          |
# keyword_runtime_thresholds  |   x    |  x    |          |
# 

# -> perfdata 
# -> status
# -> output

# # item = suite/test
def eval_state(item):
    pass

# # item = suite/test
# def eval_state(item, count=0):
# #    print count * 4 * "=" + " " + item.name
#     item_states = []
#     item_messages = []
#     item_perfdata = []
#     if hasattr(item, 'children'):
#         for subitem in item.children:
#             state, msg, perfdata = eval_state(subitem, count+1)
#             item_states.append(state)
#             item_messages.extend(msg)
#             item_perfdata.extend(perfdata)
#         if count == 0:
#             # Hier wird die Rekursion wieder verlassen.
#             # FIXME: Vergleiche nun subitem.status mit dem max(item_states): Fuehrt eine Thresholdueberschreitung zu einem anderen Status?

#             # In den Worst state soll auch der Status der Suite selbst eingehen
#             item_states.append(item.nagios_stateid)
#             worst_nagios_state = max(item_states)
#             output = "Robot Suite %s ran in %s seconds\n" % (item.name, item.elapsedtime/1000) + ", ".join(item_messages)
#             return worst_nagios_state, output, item_perfdata
#         else:
#             #return max(item_states), item_messages, item_perfdata
#             return max(item_states), ["Suite %s: %s\n" % (item.name, ", ".join(item_messages))], item_perfdata

#     else:
#         # FIXME eval thresholds! Filter by name!
#         return item.nagios_stateid, [ item.name + ": " + item.nagios_status ], [item.nagios_perfdata]

# ==============================================================================
# Classes for robot result objects =============================================
# ==============================================================================
class RFObject(object):
    RF_STATE2NAGIOSID = {
        'PASS'  : 0,
        'FAIL'  : 2
    }
    RF_STATE2NAGIOSSTATUS = {
        'PASS'  : 'OK',
        'FAIL'  : 'CRIT'
    }

    def __init__(self, level, level_relative, name, status, starttime, endtime, elapsedtime, branches=[]):
        self.level = level
        # level_relative begins at disceovery level, count separately for suites/tests/keywords
        self.level_relative = level_relative
        self.name = name
        self.status = status
        self.starttime = starttime
        self.endtime = endtime
        self.elapsedtime = elapsedtime
        # FIXME assert branches type
        self.branches = branches

    @property
    def nagios_stateid(self):
        return self.RF_STATE2NAGIOSID[self.status]

    @property
    def nagios_status(self):
        return self.RF_STATE2NAGIOSSTATUS[self.status]

    @property
    def nagios_perfdata(self):
        return ( self.name, self.elapsedtime)

    def nagios_perfdata_recursive(self):
        print "===" + self.name
        my_perfdata = [( self.name, self.elapsedtime) ] 

        my_perfdata_branches = []
        for sublist in [ subel.nagios_perfdata_recursive() for subel in self.branches ]:
            for subitem in sublist: 
                my_perfdata_branches.append(subitem)  
            
        return my_perfdata + my_perfdata_branches

class RFSuite(RFObject):
    def __init__(self, level, level_relative, name, status, starttime, endtime, elapsedtime, branches=[]):
        super(RFSuite, self).__init__(level, level_relative, name, status, starttime, endtime, elapsedtime, branches)

class RFTest(RFObject):
    def __init__(self, level, level_relative, name, status, starttime, endtime, elapsedtime, branches=[]):
        super(RFTest, self).__init__(level, level_relative, name, status, starttime, endtime, elapsedtime, branches)

class RFKeyword(RFObject):
    def __init__(self, level, level_relative, name, status, starttime, endtime, elapsedtime, branches=[]):
        super(RFKeyword, self).__init__(level, level_relative, name, status, starttime, endtime, elapsedtime, branches)
  
# ==============================================================================
# Visitor Class for Robot Result ===============================================
# ==============================================================================
class RobotMetricsVisitor(ResultVisitor):
    def __init__(self, discovery_suite_level=0):
        self.discovery_suite_level = discovery_suite_level
        self.data = []

    def visit_suite(self, suite, level=0):
        count_suites = len(suite.suites)
        # Subsuites
        if count_suites:
            subnodes = [ self.visit_suite(subsuite, level+1) for subsuite in suite.suites ]
            subobjects = RFSuite(level, level - self.discovery_suite_level, suite.name, suite.status, suite.starttime, suite.endtime, suite.elapsedtime, subnodes)
        # Testcases
        else:
            subnodes = [ self.visit_test(test, level+1) for test in suite.tests ]
            subobjects = RFSuite(level, level - self.discovery_suite_level, suite.name, suite.status, suite.starttime, suite.endtime, suite.elapsedtime, subnodes)

        if level == self.discovery_suite_level:
            self.data.append(subobjects)
        else:
            return subobjects

    def visit_test(self, test, level):
        count_keywords = len(test.keywords)
        subnodes = ()
        # A test can only contain Keywords
        if count_keywords:
            subnodes = [ self.visit_keyword(keyword, level+1, 0) for keyword in test.keywords ]

        test_node = RFTest(level, 9999, test.name, test.status, test.starttime, test.endtime, test.elapsedtime, subnodes)
        if level == self.discovery_suite_level:
            self.data.append(test_node)
        else:
            return test_node

    def visit_keyword(self, keyword, level, level_relative):
        count_keywords = len(keyword.keywords)
        subnodes = ()
        if count_keywords:
            subnodes = [ self.visit_keyword(keyword, level+1, level_relative+1) for keyword in keyword.keywords ]
        keyword_node = RFKeyword(level, level_relative, keyword.name, keyword.status, keyword.starttime, keyword.endtime, keyword.elapsedtime, subnodes)
        if level == self.discovery_suite_level:
            self.data.append(keyword_node)
        else:
            return keyword_node
      

if __name__ == "__main__":    
    global check_info
    check_info = {}

# !! robot = Section-Name!
check_info['robot'] = {
    "parse_function": parse_robot,
    "inventory_function": inventory_robot,
    "check_function": check_robot,
    "service_description": "Robot",
    "group": "robotmk",
    # FIXME
    "has_perfdata": False
}

if __name__ == "__main__":
    #import ipdb
    #ipdb.set_trace(context=5)
    global inventory_robot_rules 
    inventory_robot_rules = [
        {'condition': {}, 'value': {
            'discovery_suite_level': '2',
            'check_depth_suites': {
                'Mkdemo' : 1,
                'A-Suite1' : 1
            },
            'check_depth_keywords': {
                'Builtin.Sleep' : 1,
                'Foobar' : 2
            },
            'perfdata_creation_for': [ 'Mkdemo', 'A-suite1' ],
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
    global host_extra_conf_merged
    def host_extra_conf_merged(hostname, inventory_robot_rules):
        return inventory_robot_rules[0]['value']

    # When Debugging, we have no line list
    global xml_writeline
    def xml_writeline(f_tmpxml, line):
        f_tmpxml.write(line)


    global host_name
    def host_name():
        return "foo"

    xml = 'multisuites_wo_header'
    file = open(xml, "r")
    content = file.readlines()
    parsed = parse_robot(content)

    #ipdb.set_trace(context=5)
    inventory = inventory_robot(parsed)
    # state, msg, perfdata = check_robot("Mkdemo", [], parsed)
    state, msg, perfdata = check_robot("A-suite1", [], parsed)
    print "Debugger ended."

