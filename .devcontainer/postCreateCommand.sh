#!/bin/bash

# This step ties the workspace files with the Devcontainer. lsyncd is used to synchronize files. 
/workspaces/robotmk/.devcontainer/linkfiles.sh

# Password for the automation user
mkdir /opt/omd/sites/cmk/var/check_mk/web/automation/
echo "secret" > /opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret
chown -R cmk:cmk /opt/omd/sites/cmk/var/check_mk/web/automation


# Fire up the site
omd start

# Create localhost
/workspaces/robotmk/.devcontainer/create_dummyhost.sh