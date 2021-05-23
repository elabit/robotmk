#!/bin/bash

# This script fixes the permissions for all robotmk files which are symlinked into OMD. 
# (switching branches makes files write-protected for the site user)
# Works for CMK v1.6 and 2.x sites
# Usage: linkfiles.sh SITENAME

function main {
    MVERSION=$(cat $OMD_ROOT/.version_meta/version | cut -d '.' -f1)
    if [ $MVERSION == 1 ]; then 
        echo "Version 1"
        linkv1files
    elif [ $MVERSION == 2 ]; then 
        echo "Version 2"
        linkv2files
    else
        echo "Version $MVERSION of site $SITE is not supported by this script. Exiting."
        usage
    fi 
}

function rmpath {
    echo "clearing $LINKNAME"
    rm -rf $OMD_ROOT/$1
}

function linkpath {
    TARGET=$WORKSPACE/$1
    LINKNAME=$OMD_ROOT/$2
    echo "linking $TARGET -> $LINKNAME"
    # make sure that the link's parent dir exists
    mkdir -p $(dirname $LINKNAME)
    ln -sf $TARGET $LINKNAME
    #chmod 666 $TARGET/*
}

# Do not only symlink, but also generate needed directories. 
function link {
    echo "---"
    TARGET=$1
    LINKNAME=$2
    rmpath $LINKNAME
    linkpath $TARGET $LINKNAME
}

function linkv1files {
    # checkman
    link checkman local/share/check_mk/checkman
    # Metrics, WATO, agent_plugins
    link web_plugins local/share/check_mk/web/plugins
    link agents_plugins local/share/check_mk/agents/plugins
    #   link agent plugins also as custom package 'robotmk-external', can be 
    #   deployed with rule "deploy custom files". The V2 bakery can handle this 
    #   automatically for mode "external", but not the V1 one.
    link agents_plugins local/share/check_mk/agents/custom/robotmk-external/lib/bin
    # Check plugin dir
    link checks/v1 local/share/check_mk/checks
    # Bakery script dir
    link bakery/v1 local/share/check_mk/agents/bakery
}

function linkv2files {
    # checkman
    link checkman local/share/check_mk/checkman
    # Metrics, WATO, agent_plugins
    link web_plugins local/share/check_mk/web/plugins
    link agents_plugins local/share/check_mk/agents/plugins
    # Check plugin dir
    link checks/v2 local/lib/check_mk/base/plugins/agent_based
    # Bakery script dir
    link bakery/v2 local/lib/check_mk/base/cee/plugins/bakery
}

main