#!/usr/bin/python

check_info = {}
# f_tmpxml.write(line[0])

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

from robot.api import ExecutionResult, ResultVisitor
import tempfile
import os
from pprint import pprint


#factory_settings["robotmk_default_values"] = {
#    discovery_suite_level: 0,
#}
inventory_robot_rules = []

def parse_robot(info):
# debug
#    settings = host_extra_conf_merged(host_name(), inventory_robot_rules)
#    discovery_suite_level = settings.get("discovery_suite_level", 0)

#    print "Discovery suite level: %s" % str(discovery_suite_level)

    with tempfile.NamedTemporaryFile(delete=False) as f_tmpxml:
        for line in info:
# debug
#            f_tmpxml.write(line[0])
            f_tmpxml.write(line)
    result = ExecutionResult(f_tmpxml.name)
    # delete the tempfile
    os.remove(f_tmpxml.name)

# debug
#    suite_metrics = SuiteMetrics(int(discovery_suite_level))
    suite_metrics = SuiteMetrics(2)
    result.visit(suite_metrics)
    return suite_metrics.data


def inventory_robot(parsed):
    for suite in parsed:
        # each Suite name is a check, no default parameters (yet)
        yield suite.name, None
        # print s.name

def check_robot(item, params, parsed):
    for suite in parsed:
        if suite.name == item:
            # iteriere ab hier durch den ganzen Baum
            rc = suite.nagios_stateid
            #return rc, suite.status
            return eval_state(suite)

# item = suite/test
def eval_state(item, count=0):
#    print count * 4 * "=" + " " + item.name
    item_states = []
    item_messages = []
    item_perfdata = []
    if hasattr(item, 'children'):
        for subitem in item.children:
            state, msg, perfdata = eval_state(subitem, count+1)
            item_states.append(state)
            item_messages.extend(msg)
            item_perfdata.extend(perfdata)
        if count == 0:
            # Hier wird die Rekursion wieder verlassen.
            # FIXME: Vergleiche nun subitem.status mit dem max(item_states): Fuehrt eine Thresholdueberschreitung zu einem anderen Status?

            # In den Worst state soll auch der Status der Suite selbst eingehen
            item_states.append(item.nagios_stateid)
            worst_nagios_state = max(item_states)
            output = "Robot Suite %s ran in %s seconds\n" % (item.name, item.elapsedtime/1000) + ", ".join(item_messages)
            return worst_nagios_state, output, item_perfdata
        else:
            #return max(item_states), item_messages, item_perfdata
            return max(item_states), ["Suite %s: %s\n" % (item.name, ", ".join(item_messages))], item_perfdata

    else:
        # FIXME eval thresholds! Filter by name!
        return item.nagios_stateid, [ item.name + ": " + item.nagios_status ], [item.nagios_perfdata]

#    level=0
#    msg.append([item.nagios_stateid, item.sta)
#
#    if all([ isinstance(c, RFSuite) for c in item.children ]):
#        msg.extend(msg_iterator(item.children))
#    return msg

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


# Classes for robot result objects ==================================
class RFObject(object):
    RF_STATE2NAGIOSID = {
        'PASS'  : 0,
        'FAIL'  : 2
    }
    RF_STATE2NAGIOSSTATUS = {
        'PASS'  : 'OK',
        'FAIL'  : 'CRIT'
    }

    def __init__(self, name, status, starttime, endtime, elapsedtime):
        self.name = name
        self.status = status
        self.starttime = starttime
        self.endtime = endtime
        self.elapsedtime = elapsedtime

    @property
    def nagios_stateid(self):
        return self.RF_STATE2NAGIOSID[self.status]

    @property
    def nagios_status(self):
        return self.RF_STATE2NAGIOSSTATUS[self.status]

    @property
    def nagios_perfdata(self):
        return ( self.name, self.elapsedtime)

    def recurse_result(self, depth=None, msg=[]):
        for c in self.children:
            depth+=1
            #msg.extend(c)
            depth-=1
        return msg

class RFSuite(RFObject):
    def __init__(self, name, status, starttime, endtime, elapsedtime, children):
        self.name = name
        self.status = status
        self.starttime = starttime
        self.endtime = endtime
        self.elapsedtime = elapsedtime
        self.children = children

    @property
    def suites(self):
        if all([ isinstance(c, RFSuite) for c in self.children ]):
            return self.children
        else:
            return []

    @property
    def tests(self):
        if all([ isinstance(c, RFTest) for c in self.children ]):
            return self.children
        else:
            return []

class RFTest(RFObject):
    def __init__(self, name, status, starttime, endtime, elapsedtime):
        self.name = name
        self.status = status
        self.starttime = starttime
        self.endtime = endtime
        self.elapsedtime = elapsedtime

# Visitor Class for Robot Result =======================================
class SuiteMetrics(ResultVisitor):
    def __init__(self, discovery_suite_level=0):
        self.discovery_suite_level = discovery_suite_level
        self.data = []

    def visit_suite(self, suite, level=0):
        sep = 4*level*"-"
        #print sep + "Level %d: Suite %s (%s)" % (level, str(suite.name), str(suite.status))
        subsuitecount = len(suite.suites)
        if subsuitecount:
            subsuites = []
            for subsuite in suite.suites:
                level+=1
                subsuites.append(self.visit_suite(subsuite, level))
                level-=1
            test_object = RFSuite(suite.name, suite.status, suite.starttime, suite.endtime, suite.elapsedtime, subsuites)
        else:
            tests = []
            for subtest in suite.tests:
                tests.append(self.visit_test(subtest))
            test_object = RFSuite(suite.name, suite.status, suite.starttime, suite.endtime, suite.elapsedtime, tests)

        if level == self.discovery_suite_level:
            self.data.append(test_object)
        else:
            return test_object

    def visit_test(self,test):
        return RFTest(test.name, test.status, test.starttime, test.endtime, test.elapsedtime)



