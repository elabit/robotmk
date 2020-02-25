#!/usr/bin/python

import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument('-i', '--input', help='Robot XML input file', required=True)
parser.add_argument('-o', '--output', help='JSON output file', required=True)
arguments = parser.parse_args()

with open(arguments.input,'r') as file: 
    xmldata = file.readlines()

with open(arguments.output, 'w') as file:
    file.write(json.dumps([ [x] for x in xmldata]))

pass

