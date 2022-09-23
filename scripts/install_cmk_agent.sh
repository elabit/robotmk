#!/bin/bash

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This file installs the CMK agent on this system. 

# check if user is root
if [ "$(id -u)" != "0" ]; then
    echo "This script must be run as root" 1>&2
    exit 1
fi

echo "▹  Installing Check_MK Agent"
dpkg -i /omd/sites/cmk/var/check_mk/agents/linux_deb/localhost

echo "▹ Starting xinetd..."
nohup xinetd >/dev/null 2>&1   
