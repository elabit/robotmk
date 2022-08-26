#!/bin/bash

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This file fixed the permissions of the agent files so that the agent
# plugin can be debugged from the cmk user. 

# check if user is root
if [ "$(id -u)" != "0" ]; then
    echo "This script must be run as root" 1>&2
    exit 1
fi


# if dir exists, change permissions
if [ -d "/var/log/robotmk" ]; then
    chmod -R o+w /var/log/robotmk
    echo "Changed permissions of /var/log/robotmk:"
    ls -la /var/log/robotmk
else 
    echo "Directory /var/log/robotmk does not exist. Nothing to do."
fi
# if file is readable, change permissions
if [ -r "/etc/check_mk/robotmk.yml" ]; then
    chmod o+r /etc/check_mk/robotmk.yml
    echo "Changed permissions of /etc/check_mk/robotmk.yml:"
    ls -la /etc/check_mk/robotmk.yml
else 
    echo "File /etc/check_mk/robotmk.yml does not exist. Nothing to do."
fi
echo "Script finished."

