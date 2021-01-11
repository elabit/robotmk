#!/usr/bin/python
# This script reads the current version tag and creates a versioned robotmk MKP 

from mkp import dist
from pathlib2 import Path
import os
import sys
from shutil import copyfile, rmtree
import re
from collections import namedtuple

rootpath = Path(os.path.dirname(os.path.realpath(__file__)))

ostream = os.popen('git describe --tags')
tag = ostream.read().strip()
# match semantic version
if not re.match('^v([0-9]+)\.([0-9]+)\.([0-9]+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+)?$', tag):
    print "ERROR: Last git tag does not match the expected version format! Exiting."
    sys.exit(1)

Target = namedtuple('Target', 'path, filename')

customfiles = {
    'windows': {
        'agents/plugins/robotmk': Target('agents/custom/robotmk-windows/lib/bin', 'robotmk.py')
    },
    'linux': {
        'agents/plugins/robotmk': Target('agents/custom/robotmk-linux/bin', 'robotmk')
    },
}

for os in customfiles: 
    for file in customfiles[os]:
        custompath = customfiles[os][file].path
        Path(custompath).mkdir(parents=True, exist_ok=True)
        copyfile(file, str(Path(custompath).joinpath(customfiles[os][file].filename)))

blacklist = [
    'local/lib',
    'test_robotmk_plugin.py',
    'agents/plugins/robotmk.py',
    '__pycache__',
]

pkg_desc = '''
    RobotMK integrates Robot Framework results into CheckMK. 
    Robot Framework can test web based and native applications. 
    It is easy to learn and highly extendable by libraries. 
    Libraries provide the functionality to use modern test web 
    technologies (Playwright/Selenium), control user interfaces 
    (ImageHorizon, SikuliX, AutoIT, SAP, ...), REST/SOAP APIs 
    and many more. It is based on Python and can be extended 
    by own libraries as well. 
    See https://robotframework.org for more information.
'''


dist({
    'author': 'Simon Meggle, https://www.simon-meggle.de',
    'description': pkg_desc,
    'download_url': 'https://www.robotmk.org',
    'name': 'robotmk',
    'title': 'RobotMK | Robot Framework End2End Test Integration',
    'version': tag.replace('v',''),
    'version.min_required': '1.6',
}, blacklist=blacklist)

rmtree(str('agents/custom'))