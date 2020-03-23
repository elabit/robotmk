#!/usr/bin/python
# -*- coding: utf-8 -*-

#title           : remedy_notification.py
#description     : Script zur Erstellung von Tickets in Remedy
#author          : ext_walterro;  Anpassung/Erweiterung v. Simon Meggle <simon.meggle@elabit.de>
#date            : 01/2020
#usage       :
#notes           :
#bash_version    :

import os
import argparse

parser = argparse.ArgumentParser(description='Generate ChecMK check input from Robot XML result files')
parser.add_argument("-f", "--file", help="Robot XML result file", required=True)
arg = parser.parse_args()
try:
    infile = open(arg.file, "r")
    content = infile.readlines()
    mkcontent = [[line] for line in content]
    outfilename = "%s/%s.json" % (os.path.dirname(arg.file), os.path.splitext(os.path.basename(arg.file))[0])
    with open(outfilename, "w") as outfile:
        outfile.write(str(mkcontent))
except: 
    print "Error while converting Robot result file %s to CheckMK input" % arg.file