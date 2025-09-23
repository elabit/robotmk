#!/bin/bash

LAUNCH_TPL_FILE="$WORKSPACE/.vscode/launch_tpl.json"
TMP_FILE="$WORKSPACE/.vscode/launch.json.tmp"
TARGET_FILE="$WORKSPACE/.vscode/launch.json"
# Determine CMK major.minor for package naming (e.g., 2.2, 2.3, 2.4)
OMD_VER=$(omd version | awk '{print $NF}')
export CMK_MM=$(echo "$OMD_VER" | cut -d. -f1-2)

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