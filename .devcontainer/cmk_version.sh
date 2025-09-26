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
        2.4)
            export CMK_DIR_CHECKS="local/lib/python3/cmk_addons/plugins/robotmk/agent_based"
            export CMK_DIR_GRAPHING="local/lib/python3/cmk_addons/plugins/robotmk/graphing"
            export CMK_DIR_CHECKMAN="local/lib/python3/cmk_addons/plugins/robotmk/checkman"            
            ;;
        2.3)
            export CMK_DIR_CHECKS="local/lib/check_mk/base/plugins/agent_based"
            export CMK_DIR_GRAPHING="local/share/check_mk/web/plugins/metrics"
            export CMK_DIR_CHECKMAN="local/share/check_mk/checkman"
            ;;
        2.2)            
            export CMK_DIR_CHECKS="local/lib/check_mk/base/plugins/agent_based"
            export CMK_DIR_GRAPHING="local/share/check_mk/web/plugins/metrics"
            export CMK_DIR_CHECKMAN="local/share/check_mk/checkman"
            ;;
        *)
            # Unknown, try addons first
            echo "ERROR: Unknown CMK version: $CMK_VERSION_MM" >&2
            exit 1
            ;;
    esac

    # Stable paths across 2.2-2.4
    export CMK_DIR_AGENT_PLUGINS="local/share/check_mk/agents/plugins"
    export CMK_DIR_BAKERY="local/lib/check_mk/base/cee/plugins/bakery"
    export CMK_DIR_WATO="local/share/check_mk/web/plugins/wato"
    export CMK_DIR_IMAGES="local/share/check_mk/web/htdocs/images"
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
