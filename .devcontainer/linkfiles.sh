#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later


# This script gets called from postcreateCommand.sh directly after the devcontainer
# has been started. Its job is to make the Robotmk project files available to the CMK site.


VERBOSE=0

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

function main {
    print_workspace
    print_cmk_variables
    sync_files
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
    echo "CMK_DIR_IMAGES: $OMD_ROOT/$CMK_DIR_IMAGES"
    echo "CMK_DIR_WATO: $OMD_ROOT/$CMK_DIR_WATO"
    echo "CMK_FILE_WATO_BAKERY: $OMD_ROOT/$CMK_FILE_WATO_BAKERY"

}

function sync_files {
    echo "===================="
    echo "Syncing robotmk MKP files"
    echo "===================="

    # Get all sync targets and process them
    while IFS='|' read -r src dst type; do
        if [ -n "$src" ] && [ -n "$dst" ] && [ -n "$type" ]; then               
            sync_path "$src" "$dst" "$type"
        fi
    done < <(get_sync_targets)
    
    # Clean up Python cache files
    find "$OMD_ROOT/local" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
}

# ===============================================================
# Sync functions using rsync instead of symlinks
# ===============================================================

function sync_path {
    local SRC="$1"
    local DST="$2"
    local TYPE="$3"
    
    echo "--------------------------------"
    echo "## {WORKSPACE}/$SRC → $DST"
    
    local SOURCE_PATH="$WORKSPACE/$SRC"
    local TARGET_PATH
    
    # Handle absolute vs relative paths
    if [[ "$DST" == /* ]]; then
        TARGET_PATH="$DST"
    else
        TARGET_PATH="$OMD_ROOT/$DST"
    fi
    
    # Verify source exists
    if [ ! -e "$SOURCE_PATH" ]; then
        echo "WARNING: Source $SOURCE_PATH does not exist, skipping"
        return 1
    fi
    
    # Create parent directory for target
    mkdir -p "$(dirname "$TARGET_PATH")" 2>/dev/null || true
    
    # Sync based on type
    if [ "$TYPE" == "FOLDER" ]; then
        # For folders: use rsync with --delete to mirror exactly
        #echo "-> rsync folder (mirror mode)"
        mkdir -p "$TARGET_PATH"
        
        rsync -a --delete --exclude='__pycache__' --exclude='*.pyc' \
              "${SOURCE_PATH}/" "${TARGET_PATH}/"
        if [ $VERBOSE -eq 1 ]; then
            tree -L 2 "$TARGET_PATH" 2>/dev/null || ls -la "$TARGET_PATH"
        fi
    elif [ "$TYPE" == "FILE" ]; then        
        rsync -a "${SOURCE_PATH}" "${TARGET_PATH}"
        
        if [ $VERBOSE -eq 1 ]; then
            ls -la "$TARGET_PATH"
        fi
        echo "-> Done (file)"
    else
        echo "ERROR: Unknown sync type: $TYPE"
        return 1
    fi
    # determine how many files were synced 
    local FILE_COUNT=$(find "$SOURCE_PATH" -type f | wc -l)
    echo "-> Synced $FILE_COUNT files."    
}

function log_verbose {
    if [ $VERBOSE -eq 1 ]; then
        echo "$1"
    fi
}

main "$@"
