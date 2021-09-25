#!/bin/bash

# This step ties the workspace files with the Devcontainer. lsyncd is used to synchronize files. 
/workspaces/robotmk/.devcontainer/linkfiles.sh

echo "Extracting automation user info ... "
# tar -C /opt/omd/sites/cmk/var/check_mk/web/ -xzf /workspaces/robotmk/.devcontainer/automation.tgz

AUT_SECRET_DIR=/opt/omd/sites/cmk/var/check_mk/web/automation/

# Password for the automation user
mkdir -p $AUT_SECRET_DIR
chmod 770 $AUT_SECRET_DIR
echo "secret" > $AUT_SECRET_DIR/automation.secret
chmod 660 $AUT_SECRET_DIR/automation.secret
chown -R cmk:cmk $AUT_SECRET_DIR

# Create the folder to source agent output for easy debugging.
# Sync is done by lsyncd in linkfiles.sh.
mkdir -p /omd/sites/cmk/var/check_mk/agent_output
chown -R cmk:cmk /omd/sites/cmk/var/check_mk/agent_output

# Fire up the site
omd start

# Create localhost
/workspaces/robotmk/.devcontainer/create_dummyhost.sh