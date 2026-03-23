#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later

# Real-time file synchronization using inotifywait
# This script watches for changes in the CMK site and syncs them back to workspace
# for development workflow (CMK site → workspace one-way sync).

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

# Check for workspace
if [ -z "$WORKSPACE" ]; then
    if [ -n "$GITHUB_WORKSPACE" ]; then
        WORKSPACE="$GITHUB_WORKSPACE"
    else
        echo "ERROR: WORKSPACE is not set and GITHUB_WORKSPACE is not available"
        exit 1
    fi
fi

# Check if inotifywait is available
if ! command -v inotifywait >/dev/null 2>&1; then
    echo "ERROR: inotifywait is not installed. Install inotify-tools package."
    exit 1
fi

# PID file to track running instance
PIDFILE="/tmp/robotmk_watch_sync.pid"
LOGFILE="/tmp/robotmk_watch_sync.log"

# Cooldown to prevent sync loops
declare -A SYNC_COOLDOWN
COOLDOWN_SECONDS=2

function log_msg {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$LOGFILE"
}

function should_sync {
    local key="$1"
    local now=$(date +%s)
    local last_sync="${SYNC_COOLDOWN[$key]:-0}"
    
    if [ $((now - last_sync)) -lt $COOLDOWN_SECONDS ]; then
        return 1  # Too soon, skip
    fi
    
    SYNC_COOLDOWN[$key]=$now
    return 0  # OK to sync
}

function sync_from_cmk {
    local SRC="$1"
    local DST="$2"
    local TYPE="$3"
    
    local key="${SRC}_from_cmk"
    if ! should_sync "$key"; then
        return
    fi
    
    local SOURCE_PATH
    local TARGET_PATH="$WORKSPACE/$SRC"
    
    if [[ "$DST" == /* ]]; then
        SOURCE_PATH="$DST"
    else
        SOURCE_PATH="$OMD_ROOT/$DST"
    fi
    
    if [ ! -e "$SOURCE_PATH" ]; then
        return  # Source doesn't exist yet
    fi
    
    mkdir -p "$(dirname "$TARGET_PATH")" 2>/dev/null || true
    
    if [ "$TYPE" == "FOLDER" ]; then
        mkdir -p "$TARGET_PATH"
        rsync -a --delete --exclude='__pycache__' --exclude='*.pyc' \
              "$SOURCE_PATH/" "$TARGET_PATH/" 2>/dev/null
        log_msg "← Synced folder: $DST → $SRC"
    elif [ "$TYPE" == "FILE" ]; then
        rsync -a "$SOURCE_PATH" "$TARGET_PATH" 2>/dev/null
        log_msg "← Synced file: $DST → $SRC"
    fi
}



function start_watching {
    log_msg "Starting file watcher for Robotmk development sync"
    log_msg "CMK Version: $CMK_VERSION_MM"
    log_msg "Workspace: $WORKSPACE"
    
    # Build watch paths for CMK→workspace direction
    local -a cmk_watches=()
    local -a target_map=()
    
    while IFS='|' read -r src dst type; do
        if [ -n "$src" ] && [ -n "$dst" ] && [ -n "$type" ]; then
            local cmk_path
            if [[ "$dst" == /* ]]; then
                cmk_path="$dst"
            else
                cmk_path="$OMD_ROOT/$dst"
            fi
            if [ -e "$cmk_path" ]; then
                cmk_watches+=("$cmk_path")
                target_map+=("$src|$dst|$type")
            fi
        fi
    done < <(get_sync_targets)
    
    log_msg "Watching ${#cmk_watches[@]} paths for changes..."
    
    # Watch CMK site for changes and sync back to workspace
    while true; do
        # Use inotifywait to monitor for changes
        # Events: modify, create, delete, move
        inotifywait -q -r -m -e modify,create,delete,moved_to,moved_from \
                    --exclude '(__pycache__|\.pyc$|\.git/)' \
                    "${cmk_watches[@]}" 2>/dev/null | \
        while read -r path event file; do
            # Determine which sync target was affected
            for mapping in "${target_map[@]}"; do
                IFS='|' read -r src dst type <<< "$mapping"
                local dst_full
                if [[ "$dst" == /* ]]; then
                    dst_full="$dst"
                else
                    dst_full="$OMD_ROOT/$dst"
                fi
                
                # Check if the changed path is under this CMK target
                if [[ "$path" == "$dst_full"* ]] || [[ "$path/$file" == "$dst_full"* ]]; then
                    log_msg "Change detected: $path/$file ($event)"
                    sync_from_cmk "$src" "$dst" "$type"
                    break
                fi
            done
        done
        
        # If inotifywait exits, wait a bit and restart
        sleep 1
    done
}

function stop_watching {
    if [ -f "$PIDFILE" ]; then
        local pid=$(cat "$PIDFILE")
        if kill -0 "$pid" 2>/dev/null; then
            log_msg "Stopping file watcher (PID: $pid)"
            kill "$pid"
            rm -f "$PIDFILE"
        else
            log_msg "Stale PID file found, removing"
            rm -f "$PIDFILE"
        fi
    else
        log_msg "No watcher running"
    fi
}

function show_status {
    if [ -f "$PIDFILE" ]; then
        local pid=$(cat "$PIDFILE")
        if kill -0 "$pid" 2>/dev/null; then
            echo "File watcher is running (PID: $pid)"
            echo "Log file: $LOGFILE"
            if [ -f "$LOGFILE" ]; then
                echo "Last 10 log entries:"
                tail -n 10 "$LOGFILE"
            fi
        else
            echo "Stale PID file found, watcher not running"
        fi
    else
        echo "File watcher is not running"
    fi
}

# Main command handling
case "${1:-start}" in
    start)
        if [ -f "$PIDFILE" ]; then
            pid=$(cat "$PIDFILE")
            if kill -0 "$pid" 2>/dev/null; then
                echo "File watcher already running (PID: $pid)"
                exit 0
            fi
        fi
        
        # Start in background
        nohup "$0" _watch >> "$LOGFILE" 2>&1 &
        echo $! > "$PIDFILE"
        log_msg "File watcher started (PID: $(cat $PIDFILE))"
        ;;
    
    _watch)
        # Internal: actual watch loop
        start_watching
        ;;
    
    stop)
        stop_watching
        ;;
    
    restart)
        stop_watching
        sleep 1
        "$0" start
        ;;
    
    status)
        show_status
        ;;
    
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        echo ""
        echo "  start   - Start background file watcher"
        echo "  stop    - Stop background file watcher"
        echo "  restart - Restart file watcher"
        echo "  status  - Show watcher status and recent logs"
        exit 1
        ;;
esac
