#!/usr/bin/python

#check_info = {}

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

from robot.api import ExecutionResult, ResultVisitor
import tempfile
import os
from pprint import pprint

f_tmpxml = tempfile.NamedTemporaryFile(delete=False)

def parse_robot(info):
    for line in info:
        # CMK uses line arrays
        f_tmpxml.write(line[0])
        #f_tmpxml.write(line)
        #print line[0]
    f_tmpxml.close()
    result = ExecutionResult(f_tmpxml.name)
    # delete the tempfile
    os.remove(f_tmpxml.name)

    suite_metrics = SuiteMetrics(2)
    result.visit(suite_metrics)
    return suite_metrics.data


def inventory_robot(parsed):
    for suite in parsed:
        # each Suite name is a check, no default parameters (yet)
        yield suite.name, None
        # print s.name


def check_robot(item, params, tmpxml_name):

    warn, crit = params

    #result = ExecutionResult('/tmp/output.xml')
    result = ExecutionResult(tmpxml_name)
    for s in result.suite.suites:
        # each Suite name is a check, no default parameters (yet)
        yield s.name, None
        # print s.name
    # delete the tempfile
    os.remove(tmpxml_name)


check_info['robot'] = {
    "parse_function": parse_robot,
    "inventory_function": inventory_robot,
    "check_function": check_robot,
    "service_description": "Robot",
}





# Classes for robot result objects ==================================
class RFObject(object):
    def __init__(self, name, status, starttime, endtime, elapsedtime):
        self.name = name
        self.status = status
        self.starttime = starttime
        self.endtime = endtime
        self.elapsedtime = elapsedtime

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



