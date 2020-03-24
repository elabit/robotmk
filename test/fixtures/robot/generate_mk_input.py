#!/usr/bin/python
# -*- coding: utf-8 -*-

import os
import argparse
import json

parser = argparse.ArgumentParser(description='Generate ChecMK check input from Robot XML result files')
parser.add_argument("-f", "--file", help="Robot XML result file", required=True)
arg = parser.parse_args()
try:
    with open(arg.file, "r") as file: 
        xmldata = file.readlines()
    outfilename = "%s/%s.json" % (os.path.dirname(arg.file), os.path.splitext(os.path.basename(arg.file))[0])
    with open(outfilename, "w") as file:
        file.write(json.dumps([ [x] for x in xmldata ]))
    print "%s => %s" % (arg.file, outfilename)
except: 
    print "Error while converting Robot result file %s to CheckMK input" % arg.file

