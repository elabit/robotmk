#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

set -u
# This script gets called from postcreateCommand.sh directly after the devcontainer
# has been started. Its job is to make the Robotmk project files available to the CMK site.

L_SHARE_CMK="local/share/check_mk"
L_LIB_CMK_BASE="local/lib/check_mk/base"

function main {
    symlink_files
    echo "linkfiles.sh finished."
    echo "===================="
}

function rmpath {
    echo "clearing $1"
    rm -rf $1
}

function linkpath {
    TARGET=$WORKSPACE/$1
    LINKNAME=$2
    echo "linking $TARGET -> $LINKNAME"
    # check if target exists
    if [ ! -d $TARGET ]; then
        echo "ERROR: $TARGET does not exist!"
        exit 1
    fi
    # make sure that the link's parent dir exists
    mkdir -p $(dirname $LINKNAME)
    ln -sf $TARGET $LINKNAME
    #chmod 666 $TARGET/*
}

# Do not only symlink, but also generate needed directories.
function create_symlink {
    echo "---"
    TARGET=$1
    if [ ${2:0:1} == "/" ]; then
        # absolute link
        LINKNAME=$2
    else
        # relative link in OMD_ROOT
        LINKNAME=$OMD_ROOT/$2
    fi
    rmpath $LINKNAME
    linkpath $TARGET $LINKNAME
    tree $LINKNAME
}

function symlink_files {
    # TODO: Package linked?
    # Package File
    create_symlink pkginfo $OMD_ROOT/var/check_mk/packages/robotmk

    # Bash aliases
    create_symlink scripts/.site_bash_aliases $OMD_ROOT/.bash_aliases

    # Agent plugins
    create_symlink agents_plugins $L_SHARE_CMK/agents/plugins

    # checkman
    create_symlink checkman $L_SHARE_CMK/checkman

    # Images & icons
    create_symlink images $L_SHARE_CMK/web/htdocs/images

    # Metrics, WATO
    create_symlink web_plugins $L_SHARE_CMK/web/plugins

    # # RF test suites
    create_symlink rf_tests /usr/lib/check_mk_agent/robot
    # Folder where agent output can be sourced with rule
    # "Datasource Programs > Individual program call instead of agent access"
    # (folder gets created in postCreateCommand.sh)
    create_symlink agent_output var/check_mk/agent_output

    # BAKERY
    create_symlink bakery ${L_LIB_CMK_BASE}/cee/plugins/bakery
    rm -rf ${L_LIB_CMK_BASE}/cee/plugins/bakery/__pycache__

    # CHECK PLUGIN
    create_symlink checks ${L_LIB_CMK_BASE}/plugins/agent_based
    rm -rf ${L_LIB_CMK_BASE}/plugins/agent_based/__pycache__
}

main
