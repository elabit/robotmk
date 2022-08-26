#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

set -u
# This script gets called from postcreateCommand.sh directly after the devcontainer
# has been started. Its job is to make the Robotmk project files available to the CMK site.

#exit

L_SHARE_CMK="local/share/check_mk"
L_LIB_CMK_BASE="local/lib/check_mk/base"

function main {
    # Detect major version and decide what to link
    MVERSION=$(cat $OMD_ROOT/.version_meta/version | cut -d '.' -f1)
    echo -n "Site $OMD_SITE: "
    if [ $MVERSION == 1 ]; then 
        echo "Detected CMK major version 1"
        sync_v1files
    elif [ $MVERSION == 2 ]; then 
        echo "Detected CMK major version 2"
        sync_v2files
    else
        echo "Detected CMK major version $MVERSION is not supported by this script (only 1 and 2). Exiting."
        usage
    fi 
    sync_common
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

function sync_common {
    # Bash aliases
    create_symlink scripts/.bash_aliases $OMD_ROOT/.bash_aliases
    
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
}

function sync_v1files {
    # CUSTOM PACKAGE 'robotmk-external' (install with rule "deploy custom files"). 
    # (the V2 bakery can handle this automatically for mode "external")
    create_symlink agents_plugins $L_SHARE_CMK/agents/custom/robotmk-external/lib/bin

    # BAKERY V1
    create_symlink bakery/v1 $L_SHARE_CMK/agents/bakery

    # CHECK PLUGIN V1
    create_symlink checks/v1 $L_SHARE_CMK/checks
}

function sync_v2files {

    # Custom package "robotmk-external"
    # - not needed in V2 - 
    # Bakery script dir
    
    # BAKERY V2
    create_symlink bakery/v2 $L_SHARE_CMK/agents/bakery
    rm -rf $L_SHARE_CMK/agents/bakery/__pycache__

    # CHECK PLUGIN V2
    create_symlink checks/v2 $L_LIB_CMK_BASE/plugins/agent_based
    rm -rf $L_LIB_CMK_BASE/plugins/agent_based/__pycache__ 
}

main


# -----------------------------------------------------------------------------


# Synchronize a certain folder with lsyncd: 
# - do an initial rsync from the workspace into the dest dir
# - write the lsyncd config for a two-way-sync 
function lsync_this {
    echo "---"
    SOURCE=$WORKSPACE/$1
    
    if [ ${2:0:1} == "/" ]; then 
        DEST=$2
    else
        DEST=$OMD_ROOT/$2
    fi
    echo "Workspace dir: $SOURCE"
    echo "Container dir: $DEST"
    echo "> writing lsync config... "
    echo "-- $DEST" >> $LSYNCD_CFG
    mkdir -p $DEST
    rsync --quiet -ap $SOURCE/ $DEST
    write_lsync_cfg $SOURCE $DEST
    write_lsync_cfg $DEST $SOURCE
    echo >> $LSYNCD_CFG
    # uncomment this to debug the script and see if the initial sync was successful:
    #ls -la $DEST 
}

function write_lsync_cfg {
    SOURCE=$1
    DEST=$2
    cat <<EOF >> $LSYNCD_CFG
sync {
  default.rsync,
  source = "$SOURCE",
  target = "$DEST",
  delay = 1,
}
EOF
} 

# Global lsyncd settings
function write_lsyncd_header {
    cat > $OMD_ROOT/.lsyncd << "EOF"
settings {
   inotifyMode = "CloseWrite or Modify"
}


EOF
}