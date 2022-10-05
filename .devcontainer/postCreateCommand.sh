#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)


echo "▹ WORKSPACE: $WORKSPACE"
# This step ties the workspace files with the Devcontainer. lsyncd is used to synchronize files. 
echo "▹ Linking the project files into the container (linkfiles.sh)..."
/workspaces/robotmk/.devcontainer/linkfiles.sh

# Tell bash to load aliases and functions
echo "▹ Loading aliases and functions..."
echo ". $HOME/.bash_aliases" >> $HOME/.bashrc

# Create the folder to source agent output for easy debugging.
# Sync is done by lsyncd in linkfiles.sh.
mkdir -p $OMD_ROOT/var/check_mk/agent_output
chown -R cmk:cmk $OMD_ROOT/var/check_mk/agent_output

echo "▹ Setting automation user secret to 'secret' ... "
AUT_SECRET_DIR=$OMD_ROOT/var/check_mk/web/automation/
mkdir -p $AUT_SECRET_DIR
chmod 770 $AUT_SECRET_DIR
echo "secret" > $AUT_SECRET_DIR/automation.secret
chmod 660 $AUT_SECRET_DIR/automation.secret
chown -R cmk:cmk $AUT_SECRET_DIR

echo "▹ Disabling the EC..."
sed -i '/mkeventd_enabled/d' $OMD_ROOT/etc/check_mk/conf.d/mkeventd.mk
echo "mkeventd_enabled = False" >> $OMD_ROOT/etc/check_mk/conf.d/mkeventd.mk
sed -i '/mkeventd_enabled/d' $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk
echo "mkeventd_enabled = False" >> $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk


echo "▹ Disabling the Liveproxyd..."
sed -i '/liveproxyd_enabled/d' $OMD_ROOT/etc/check_mk/multisite.d/mkeventd.mk
echo "liveproxyd_enabled = False" >> $OMD_ROOT/etc/check_mk/multisite.d/mkeventd.mk

echo "▹ Enabling the Web API..."
sed -i '/disable_web_api/d' $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk
echo "disable_web_api = False" >> $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk

echo "▹ Installing Python modules for Robotmk... "
pip3 install -r /workspaces/robotmk/requirements.txt

echo "▹ Starting OMD... "
omd restart

echo "▹ Creating localhost via Web API..."
/workspaces/robotmk/.devcontainer/create_dummyhost.sh

echo "✅ postCreateCommand.sh finished."