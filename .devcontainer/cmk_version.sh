#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later

# Utility functions for Checkmk version detection and target path resolution
# This script can be sourced by other scripts to access CMK version information
# and target path variables based on the detected CMK version.

# Determine CMK major.minor version (e.g., 2.2, 2.3, 2.4)
# Prefer environment CMK_VERSION if provided; else detect from omd
function detect_cmk_version() {
    local detected_version="${CMK_VERSION:-}"
    if [ -z "$detected_version" ]; then
        # Check if omd command is available
        if ! command -v omd >/dev/null 2>&1; then
            echo "ERROR: omd command not available and CMK_VERSION not set" >&2
            return 1
        fi
        # Extract last field, then cut major.minor
        # Example: 2.4.0p1.cee => 2.4
        local omd_ver
        omd_ver=$(omd version | awk '{print $NF}')
        detected_version=$(echo "$omd_ver" | cut -d. -f1-2)
    fi
    
    echo "$detected_version"
}

# Set global CMK_VERSION_MM variable and export all target variables
function resolve_cmk_targets() {
    CMK_VERSION_MM=$(detect_cmk_version)
    if [ $? -ne 0 ] || [ -z "$CMK_VERSION_MM" ]; then
        echo "ERROR: Failed to detect CMK version" >&2
        return 1
    fi
    export CMK_VERSION_MM
    
    case "$CMK_VERSION_MM" in
        2.5)
            export CMK_DIR_CHECKS="local/lib/python3/cmk_addons/plugins/robotmk/agent_based"
            export CMK_DIR_GRAPHING="local/lib/python3/cmk_addons/plugins/robotmk/graphing"
            export CMK_DIR_CHECKMAN="local/lib/python3/cmk_addons/plugins/robotmk/checkman"                    
            export CMK_DIR_BAKERY="local/lib/python3/cmk/base/cee/plugins/bakery"
            export CMK_FILE_WATO_BAKERY="local/lib/python3/cmk_addons/plugins/robotmk/rulesets/robotmk_wato_params_bakery.py"
            export CMK_FILE_WATO_DISCOVERY="local/lib/python3/cmk_addons/plugins/robotmk/rulesets/robotmk_wato_params_discovery.py"
            export CMK_FILE_WATO_CHECK="local/lib/python3/cmk_addons/plugins/robotmk/rulesets/robotmk_wato_params_check.py"
            ;;
        2.4)
            export CMK_DIR_CHECKS="local/lib/python3/cmk_addons/plugins/robotmk/agent_based"
            export CMK_DIR_GRAPHING="local/lib/python3/cmk_addons/plugins/robotmk/graphing"
            export CMK_DIR_CHECKMAN="local/lib/python3/cmk_addons/plugins/robotmk/checkman"            
            export CMK_DIR_BAKERY="local/lib/check_mk/base/cee/plugins/bakery"
            export CMK_FILE_WATO_BAKERY="local/share/check_mk/web/plugins/wato/robotmk_wato_params_bakery.py"
            export CMK_FILE_WATO_DISCOVERY="local/share/check_mk/web/plugins/wato/robotmk_wato_params_discovery.py"
            export CMK_FILE_WATO_CHECK="local/share/check_mk/web/plugins/wato/robotmk_wato_params_check.py"
            ;;
        2.3)
            export CMK_DIR_CHECKS="local/lib/python3/cmk_addons/plugins/robotmk/agent_based"
            export CMK_DIR_GRAPHING="local/lib/python3/cmk_addons/plugins/robotmk/graphing"
            export CMK_DIR_CHECKMAN="local/lib/python3/cmk_addons/plugins/robotmk/checkman"            
            export CMK_DIR_BAKERY="local/lib/check_mk/base/cee/plugins/bakery"
            export CMK_FILE_WATO_BAKERY="local/share/check_mk/web/plugins/wato/robotmk_wato_params_bakery.py"
            export CMK_FILE_WATO_DISCOVERY="local/share/check_mk/web/plugins/wato/robotmk_wato_params_discovery.py"
            export CMK_FILE_WATO_CHECK="local/share/check_mk/web/plugins/wato/robotmk_wato_params_check.py"
            ;;
        *)
            # Unknown, try addons first
            echo "ERROR: Unknown CMK version: $CMK_VERSION_MM" >&2
            exit 1
            ;;
    esac

    # Stable paths
    export CMK_DIR_AGENT_PLUGINS="local/share/check_mk/agents/plugins"
    export CMK_DIR_WATO="local/share/check_mk/web/plugins/wato"
    export CMK_DIR_IMAGES="local/share/check_mk/web/htdocs/images"
}

# Define sync targets with their types (FOLDER or FILE)
# Format: "workspace_path|target_path|type"
function get_sync_targets() {
    local -a targets=()
    
    # Robotmk check plugins - FOLDER sync
    targets+=("checks|${CMK_DIR_CHECKS}|FOLDER")
    
    # Robotmk metrics/graphing - FOLDER sync
    targets+=("web_plugins/metrics|${CMK_DIR_GRAPHING}|FOLDER")
    
    # Robotmk checkman - FOLDER sync
    targets+=("checkman|${CMK_DIR_CHECKMAN}|FOLDER")
    
    # Robotmk bakery - FOLDER sync
    targets+=("bakery|${CMK_DIR_BAKERY}|FOLDER")
    
    # WATO rules 
    #targets+=("web_plugins/wato|local/share/check_mk/web/plugins/wato|FOLDER")

    # WATO bakery ruleset - FILE sync
    targets+=("web_plugins/wato/robotmk_wato_params_bakery.py|${CMK_FILE_WATO_BAKERY}|FILE")
    # WATO discovery ruleset - FILE sync
    targets+=("web_plugins/wato/robotmk_wato_params_discovery.py|${CMK_FILE_WATO_DISCOVERY}|FILE")
    # WATO check ruleset - FILE sync
    targets+=("web_plugins/wato/robotmk_wato_params_check.py|${CMK_FILE_WATO_CHECK}|FILE")


    # Agent plugins - folder
    targets+=("agents_plugins|${CMK_DIR_AGENT_PLUGINS}|FOLDER")

    # Images/icons
    targets+=("images|${CMK_DIR_IMAGES}|FOLDER")
    
    # Common files
    targets+=("scripts/.site_bash_aliases|.bash_aliases|FILE")
    targets+=("rf_tests|/usr/lib/check_mk_agent/robot|FOLDER")
    targets+=("agent_output|var/check_mk/agent_output|FOLDER")    
    
    printf "%s\n" "${targets[@]}"
}

# Auto-resolve targets when script is sourced (for convenience)
if ! resolve_cmk_targets; then
    # If we're being sourced and resolution fails, it's likely we're not in a CMK environment
    # Only error if we're running standalone
    if [ "${BASH_SOURCE[0]}" == "${0}" ]; then
        exit 1
    fi
fi

# Export the functions for other scripts to use
export -f detect_cmk_version
export -f resolve_cmk_targets
export -f get_sync_targets
