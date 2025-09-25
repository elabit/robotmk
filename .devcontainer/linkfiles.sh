#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later


# This script gets called from postcreateCommand.sh directly after the devcontainer
# has been started. Its job is to make the Robotmk project files available to the CMK site.

# The script can be run in two modes: 
# ./linkfiles.sh cmkonly => link only the CMK relevant files (bash aliases etc)
# ./linkfiles.sh full => link the robotmk files as well as additional files

L_SHARE_CMK="local/share/check_mk"
L_LIB_CMK_BASE="local/lib/check_mk/base"
L_LIB_PY3_CMK_ADDONS="local/lib/python3/cmk_addons"

# Determine CMK major.minor version (e.g., 2.2, 2.3, 2.4)
# Prefer environment CMK_VERSION if provided; else detect from omd
CMK_VERSION_MM="${CMK_VERSION:-}"
if [ -z "$CMK_VERSION_MM" ]; then
    # Extract last field, then cut major.minor
    # Example: 2.4.0p1.cee => 2.4
    OMD_VER=$(omd version | awk '{print $NF}')
    CMK_VERSION_MM=$(echo "$OMD_VER" | cut -d. -f1-2)
fi

function _resolve_targets() {
    case "$CMK_VERSION_MM" in
        2.4)
            TARGET_CHECKS="$L_LIB_PY3_CMK_ADDONS/plugins/robotmk/agent_based"
            TARGET_GRAPHING="${L_LIB_PY3_CMK_ADDONS}/plugins/robotmk/graphing"
            ;;
        2.3)
            TARGET_CHECKS="$L_LIB_PY3_CMK_ADDONS/plugins/robotmk/agent_based"
            TARGET_GRAPHING="${L_LIB_PY3_CMK_ADDONS}/plugins/robotmk/graphing"
            ;;
        2.2)
            # 2.2 may not ship cmk_addons by default; prefer it if present, else legacy
            TARGET_CHECKS="$L_LIB_CMK_BASE/plugins/agent_based"
            TARGET_GRAPHING="${L_SHARE_CMK}/web/plugins/metrics"
            ;;
        *)
            # Unknown, try addons first
            echo "ERROR: Unknown CMK version: $CMK_VERSION_MM"
            exit 1
            ;;
    esac

    # Bakery path has been stable across 2.2-2.4
    TARGET_BAKERY="$L_LIB_CMK_BASE/cee/plugins/bakery"
    TARGET_WATO="$L_SHARE_CMK/web/plugins/wato"
    TARGET_IMAGES="$L_SHARE_CMK/web/htdocs/images"
    
}

_resolve_targets

# check for Argument
if [ -z "$1" ]; then
    echo "ERROR: Argument must be either 'cmkonly' or 'full'."
    exit 1
else
    ARG1="$1"
fi

# ARG1 must be either "cmkonly" or "full"
if [ "$ARG1" != "cmkonly" ] && [ "$ARG1" != "full" ]; then
    echo "ERROR: Argument must be either 'cmkonly' or 'full'."
    exit 1
fi

function main {
    echo "Workspace: $WORKSPACE"
    #ls -la "$WORKSPACE"    
    symlink_robotmk
    symlink_files
    echo "linkfiles.sh finished."
    echo "===================="
}

function symlink_robotmk {
    if [ "$ARG1" == "full" ]; then
        echo "===================="
        echo "Linking robotmk MKP files"
        echo "===================="

        # Robotmk Package Directory
        # create_symlink pkginfo $OMD_ROOT/var/check_mk/packages

        # Robotmk Agent plugins
        create_symlink agents_plugins $L_SHARE_CMK/agents/plugins

        # Robotmk checkman
        create_symlink checkman $L_LIB_PY3_CMK_ADDONS/plugins/robotmk/checkman

        # Robotmk Images & icons
        create_symlink images $TARGET_IMAGES

        # Robotmk Metrics
        create_symlink web_plugins/metrics $TARGET_GRAPHING
        
        # WATO Rules
        create_symlink web_plugins/wato $TARGET_WATO

        # Robotmk BAKERY
        create_symlink bakery $TARGET_BAKERY
        rm -rf ${L_LIB_CMK_BASE}/cee/plugins/bakery/__pycache__

        # Robotmk CHECK PLUGIN 
        create_symlink checks $TARGET_CHECKS
        
        rm -rf ${L_LIB_PY3_CMK_ADDONS}/plugins/agent_based/__pycache__ || true
        rm -rf ${L_LIB_CMK_BASE}/plugins/agent_based/__pycache__ || true

    fi
}

function symlink_files {
    echo "===================="
    echo "Linking CMK common files"
    echo "===================="

    # Bash aliases
    create_symlink scripts/.site_bash_aliases $OMD_ROOT/.bash_aliases
    


    # # RF test suites
    create_symlink rf_tests /usr/lib/check_mk_agent/robot
    # Folder where agent output can be sourced with rule
    # "Datasource Programs > Individual program call instead of agent access"
    # (folder gets created in postCreateCommand.sh)
    create_symlink agent_output var/check_mk/agent_output

}

# ===============================================================


function rmpath {
    echo "clearing $1"
    rm -rf $1
}


function linkpath {
    TARGET=$WORKSPACE/$1
    LINKNAME=$2
    echo "linking $TARGET -> $LINKNAME"
    # check if target file or dir exists
    if [ ! -e $TARGET ]; then
        echo "ERROR: $TARGET does not exist!"
        exit 1
    fi

    # make sure that the link's parent dir exists
    mkdir -p $(dirname $LINKNAME)
    ln -sf $TARGET $LINKNAME
    # if target is a dir, show tree
    if [ -d $TARGET ]; then
        echo "Directory:"
        tree $LINKNAME
    else
        echo "File:"
        ls -la $LINKNAME
    fi
    #chmod 666 $TARGET/*
}

# Do not only symlink, but also generate needed directories.
function create_symlink {
    echo "--------------------------------"
    echo "## $1"
    echo ""
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
}

main "$@"
