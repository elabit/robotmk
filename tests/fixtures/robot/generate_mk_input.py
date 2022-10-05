#!/usr/bin/python
# -*- coding: utf-8 -*-

import os
import argparse
import json
from shutil import copyfile
import codecs

parser = argparse.ArgumentParser(description='Generate ChecMK agent and check input from Robot XML result files')
parser.add_argument("-f", "--file", help="Robot XML result file", required=True)
# default='/var/lib/check_mk_agent/spool'
parser.add_argument("-s", "--spooldir", help="CMK spooldir", required=False)
arg = parser.parse_args()
filename = arg.file.replace('./', '')
try:
    with codecs.open(filename, "r", 'utf-8') as file: 
        xmldata = file.readlines()
    suitename = os.path.dirname(filename).split('/')[0]
    print "=== %s " % suitename
    # create spool file for agent    
    agent_input_filename = "%s/input_agent.cmk" % suitename
    print "  -> %s" % agent_input_filename
    with codecs.open(agent_input_filename, "w", 'utf-8') as file:
        file.write('<<<robotmk:sep(0)>>>\n')
        file.writelines(xmldata)
    if arg.spooldir: 
        spoolfile = '%s/robotmk_%s' % (arg.spooldir, suitename)
        copyfile(agent_input_filename, spoolfile)
        print "  -> %s" % spoolfile

    # create data which Checkmk passes to the check ('list of lists')
    check_input_filename = "%s/input_check.json" % suitename
    print "  -> %s" % check_input_filename
    with codecs.open(check_input_filename, "w", 'utf-8') as file:
        json.dump([ [x] for x in xmldata ], file, ensure_ascii=False)
except: 
    print "Error while converting Robot result file %s to Checkmk input" % filename

