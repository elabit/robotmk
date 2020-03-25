#!/usr/bin/python
# -*- coding: utf-8 -*-

import os
import argparse
import json
from shutil import copyfile

parser = argparse.ArgumentParser(description='Generate ChecMK agent and check input from Robot XML result files')
parser.add_argument("-f", "--file", help="Robot XML result file", required=True)
parser.add_argument("-s", "--spooldir", help="CMK spooldir", default='/var/lib/check_mk_agent/spool', required=False)
arg = parser.parse_args()
filename = arg.file.replace('./', '')
try:
    with open(filename, "r") as file: 
        xmldata = file.readlines()
    suitename = os.path.dirname(filename)
    print "=== %s " % suitename
    # create spool file for agent    
    agent_input_filename = "%s/input_agent.cmk" % suitename
    print "  -> %s" % agent_input_filename
    with open(agent_input_filename, "w") as file:
        file.write('<<<robotmk:sep(0)>>>\n')
        file.writelines(xmldata)
    spoolfile = '%s/robotmk_%s' % (arg.spooldir, suitename)
    copyfile(agent_input_filename, spoolfile)
    print "  -> %s" % spoolfile

    # create data which CheckMK passes to the check ('list of lists')
    check_input_filename = "%s/input_check.json" % suitename
    print "  -> %s" % check_input_filename
    with open(check_input_filename, "w") as file:
        file.write(json.dumps([ [x] for x in xmldata ]))
except: 
    print "Error while converting Robot result file %s to CheckMK input" % filename

