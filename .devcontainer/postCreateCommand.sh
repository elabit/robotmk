#!/bin/bash

.devcontainer/linkfiles.sh
omd start

SECRET=$(cat /opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret)
HOST=localhost:5000
SITE=cmk

curl -k "http://$HOST/$SITE/check_mk/webapi.py?action=add_host&_username=automation&_secret=$SECRET&request_format=python&output_format=python" -d "request={'hostname': 'win10simdows', 'folder': '', 'attributes': {'ipaddress': '192.168.116.8'}, 'create_folders': '1'}"
cmk -IIv win10simdows
cmk -R