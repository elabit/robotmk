#!/bin/bash

# This step ties the workspace files with the Devcontainer. lsyncd is used to synchronize files. 
/workspaces/robotmk/.devcontainer/linkfiles.sh

# Password for the automation user
echo "secret" > /opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret

# Fire up the site
omd start