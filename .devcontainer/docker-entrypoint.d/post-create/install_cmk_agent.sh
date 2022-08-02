#!/bin/bash
# This script gets executed as a hook after the Docker entrypoint script has 
# created the OMD site.  
# Note: the agent installed here has no relation to the CMK version in this container. 
# As agent installers are only available after the first login into the site, 
# we do not have access to them. Instead, a recent deb gets installed. Will work
# for most needs...  
# As soon as the first installer has been baken by the bakery, the agent will 
# anyhow have a version from the CMK server.  

echo "▹ Installing the Checkmk agent..."
DEB=$(realpath $(dirname $0))/cmk_agent.deb
dpkg -i $DEB

ln -sf /var/log/robotmk /rmk_log
ln -sf /usr/lib/check_mk_agent/plugins /cmk_plugins
ln -sf /usr/lib/check_mk_agent/robot /cmk_robotdir
ln -sf /etc/check_mk/robotmk.yml /rmk_yml

echo "▹ Starting the Checkmk agent..."
# nohup xinetd 2>&1 > /dev/null
nohup xinetd 2>&1