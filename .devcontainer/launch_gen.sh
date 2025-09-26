#!/bin/bash

# Source CMK version detection utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cmk_version.sh"

LAUNCH_TPL_FILE="$WORKSPACE/.vscode/launch_tpl.json"
TMP_FILE="$WORKSPACE/.vscode/launch.json.tmp"
TARGET_FILE="$WORKSPACE/.vscode/launch.json"
# Use CMK_VERSION_MM from the shared utility
export CMK_MM="$CMK_VERSION_MM"

# check if WORKSPACE is set
if [ -z "$WORKSPACE" ]; then
    echo "WORKSPACE is not set"
    exit 1
fi

# check if LAUNCH_TPL_FILE exists
if [ ! -f "$LAUNCH_TPL_FILE" ]; then
    echo "LAUNCH_TPL_FILE does not exist"
    exit 1
fi


envsubst < "$LAUNCH_TPL_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$TARGET_FILE"
echo ">>> VS Code launch file launch.json created."