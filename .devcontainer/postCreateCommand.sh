#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

LINKTYPE=$1
# ARG1 must be either "cmkonly" or "full" => linkfiles.sh
if [ "$LINKTYPE" != "cmkonly" ] && [ "$LINKTYPE" != "full" ]; then
    echo "ERROR: Argument must be either 'common' or 'full'."
    exit 1
fi


echo "▹ WORKSPACE: $WORKSPACE"
# This step ties the workspace files with the Devcontainer. lsyncd is used to synchronize files. 
echo "▹ Linking the project files into the container (linkfiles.sh $LINKTYPE)..."
/workspaces/robotmk/.devcontainer/linkfiles.sh $LINKTYPE

# Tell bash to load aliases and functions
echo "▹ Loading aliases and functions..."
echo ". $HOME/.bash_aliases" >> $HOME/.bashrc

# Create the folder to source agent output for easy debugging.
# Sync is done by lsyncd in linkfiles.sh.
mkdir -p $OMD_ROOT/var/check_mk/agent_output
chown -R cmk:cmk $OMD_ROOT/var/check_mk/agent_output

echo "▹ Disabling the EC..."
sed -i '/mkeventd_enabled/d' $OMD_ROOT/etc/check_mk/conf.d/mkeventd.mk
echo "mkeventd_enabled = False" >> $OMD_ROOT/etc/check_mk/conf.d/mkeventd.mk
sed -i '/mkeventd_enabled/d' $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk
echo "mkeventd_enabled = False" >> $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk


echo "▹ Disabling the Liveproxyd..."
sed -i '/liveproxyd_enabled/d' $OMD_ROOT/etc/check_mk/multisite.d/mkeventd.mk
echo "liveproxyd_enabled = False" >> $OMD_ROOT/etc/check_mk/multisite.d/mkeventd.mk

#echo "▹ Enabling the Web API..."
#sed -i '/disable_web_api/d' $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk
#echo "disable_web_api = False" >> $OMD_ROOT/etc/check_mk/multisite.d/wato/global.mk

echo "▹ Installing Python modules for Robotmk... "
pip3 install -r /workspaces/robotmk/requirements.txt

echo "▹ Starting OMD... "
omd restart

#echo "▹ Creating dummyhost via Web API..."
#/workspaces/robotmk/.devcontainer/create_dummyhost.sh
#echo "✅ postCreateCommand.sh finished."

echo "To create a dummy host, first create an automation user with administrator rights and store the secret in clear text!"