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

# Source CMK version detection and target resolution utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cmk_version.sh"

if [ -f /omd/sites/cmk/.profile ]; then
    set -a
    . /omd/sites/cmk/.profile
    set +a
else 
    echo "ERROR: .profile not found in /omd/sites/cmk. Exiting."
    exit 1
fi

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
    print_workspace
    print_cmk_variables
    symlink_robotmk
    symlink_files
    echo "linkfiles.sh finished."
    echo "===================="
}

function print_workspace {
    if [ -z "$WORKSPACE" ]; then
        if [ -n "$GITHUB_WORKSPACE" ]; then
            WORKSPACE="$GITHUB_WORKSPACE"
        else
            echo "ERROR: WORKSPACE is not set and GITHUB_WORKSPACE is not available"
            exit 1
        fi
    fi
    echo "Workspace folder: $WORKSPACE"
    #ls -la "$WORKSPACE"
}

function print_cmk_variables {
    echo "Variables:"
    echo "=========="
    echo "CMK_DIR_CHECKS: $OMD_ROOT/$CMK_DIR_CHECKS"
    echo "CMK_DIR_GRAPHING: $OMD_ROOT/$CMK_DIR_GRAPHING"
    echo "CMK_DIR_CHECKMAN: $OMD_ROOT/$CMK_DIR_CHECKMAN"
    echo "CMK_DIR_AGENT_PLUGINS: $OMD_ROOT/$CMK_DIR_AGENT_PLUGINS"
    echo "CMK_DIR_BAKERY: $OMD_ROOT/$CMK_DIR_BAKERY"
    echo "CMK_DIR_WATO: $OMD_ROOT/$CMK_DIR_WATO"
    echo "CMK_DIR_IMAGES: $OMD_ROOT/$CMK_DIR_IMAGES"
}

function symlink_robotmk {
    if [ "$ARG1" == "full" ]; then
        echo "===================="
        echo "Linking robotmk MKP files"
        echo "===================="

        # Robotmk CHECK PLUGIN 
        create_symlink checks $CMK_DIR_CHECKS

        # Robotmk Metrics
        create_symlink web_plugins/metrics $CMK_DIR_GRAPHING

        # Robotmk checkman
        create_symlink checkman $CMK_DIR_CHECKMAN

        # stable paths across 2.2-2.4
        # Robotmk Agent plugins
        create_symlink agents_plugins $CMK_DIR_AGENT_PLUGINS

        # Robotmk BAKERY
        create_symlink bakery $CMK_DIR_BAKERY

        # WATO Rules
        create_symlink web_plugins/wato $CMK_DIR_WATO

        # Robotmk Images & icons
        create_symlink images $CMK_DIR_IMAGES
        
        
        rm -rf local/lib/python3/cmk_addons/plugins/agent_based/__pycache__ || true
        rm -rf local/lib/check_mk/base/plugins/agent_based/__pycache__ || true
        rm -rf local/lib/check_mk/base/cee/plugins/bakery/__pycache__ || true

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
    echo "clearing $LINKNAME/__pycache__"
    rm -rf $LINKNAME/__pycache__ || true
}

main "$@"
