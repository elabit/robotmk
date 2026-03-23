# Robotmk File Synchronization System

## Overview

Starting with CMK 2.5, symlinks inside the devcontainer cause issues. The new synchronization system uses **rsync** for file copying with optional real-time watching for development.

## Components

### 1. `cmk_version.sh`
Detects CMK version and exports sync target definitions with type information (FOLDER or FILE).

**Key functions:**
- `detect_cmk_version()` - Detects CMK major.minor version
- `resolve_cmk_targets()` - Exports CMK version-specific paths
- `get_sync_targets()` - Returns list of sync targets in format: `source|destination|type`

### 2. `linkfiles.sh`
Performs initial one-time sync from workspace to CMK directories using rsync.

**Usage:**
```bash
./linkfiles.sh cmkonly   # Sync only CMK common files
./linkfiles.sh full      # Sync all Robotmk MKP files + common files
```

**How it works:**
- Reads sync targets from `get_sync_targets()`
- For FOLDER targets: `rsync -a --delete source/ dest/` (mirror mode)
- For FILE targets: `rsync -a source dest`
- Excludes `__pycache__` and `*.pyc` files

### 3. `watch_sync.sh` (Optional - Requires inotify-tools)
Provides real-time bidirectional file synchronization for live development.

**Prerequisites:**
```bash
# Install inotify-tools (if not in devcontainer)
sudo apt-get install inotify-tools
```

**Usage:**
```bash
./watch_sync.sh start    # Start background file watcher
./watch_sync.sh stop     # Stop background file watcher
./watch_sync.sh restart  # Restart file watcher
./watch_sync.sh status   # Show status and recent logs
```

**How it works:**
- Uses `inotifywait` to monitor workspace for file changes
- Auto-syncs changes from workspace → CMK directories
- Includes cooldown mechanism to prevent sync loops
- Logs activity to `/tmp/robotmk_watch_sync.log`

## Migration from Symlink-based Approach

### Before (CMK ≤ 2.4):
```bash
ln -sf /workspaces/robotmk/checks /omd/sites/cmk/local/lib/.../agent_based
```

### After (CMK 2.5+):
```bash
rsync -a --delete /workspaces/robotmk/checks/ /omd/sites/cmk/local/lib/.../agent_based/
```

## Workflow

### Initial Setup (Done by `postCreateCommand.sh`):
1. Source `cmk_version.sh` to detect CMK version and get sync targets
2. Run `linkfiles.sh full` to perform initial sync
3. Start `watch_sync.sh` for real-time development sync (if inotify-tools installed)

### Development Workflow:
1. Edit files in `/workspaces/robotmk/` (Git workspace)
2. File watcher auto-syncs changes to CMK directories
3. Test in CMK
4. Commit changes from workspace (Git still works normally)

### Manual Sync (if watcher not running):
```bash
cd /workspaces/robotmk
./.devcontainer/linkfiles.sh full
```

## Sync Targets by CMK Version

### CMK 2.5:
- Checks: `local/lib/python3/cmk_addons/plugins/robotmk/agent_based/`
- Graphing: `local/lib/python3/cmk_addons/plugins/robotmk/graphing/`
- Checkman: `local/lib/python3/cmk_addons/plugins/robotmk/checkman/`
- Bakery: `local/lib/python3/cmk/base/cee/plugins/bakery/`
- WATO (individual files):
  - `robotmk_wato_params_bakery.py`
  - `robotmk_wato_params_discovery.py`
  - `robotmk_wato_params_check.py`

### CMK 2.2-2.4:
- Similar structure with version-specific paths
- See `cmk_version.sh` for exact mappings

## Troubleshooting

### Files not syncing:
```bash
# Check if watcher is running
./watch_sync.sh status

# Manual sync
./linkfiles.sh full

# Restart watcher
./watch_sync.sh restart
```

### Check sync logs:
```bash
tail -f /tmp/robotmk_watch_sync.log
```

### Verify sync targets:
```bash
source .devcontainer/cmk_version.sh
get_sync_targets
```

## Benefits

✅ **CMK 2.5 Compatible** - No symlink issues  
✅ **Git-friendly** - Files stay in workspace  
✅ **Real-time sync** - Auto-sync during development (with watcher)  
✅ **Bidirectional** - Can edit in either location  
✅ **Version-aware** - Adapts to CMK 2.2, 2.3, 2.4, 2.5  
✅ **Clean** - Automatic __pycache__ exclusion  

## Notes

- Rsync preserves file permissions and timestamps
- `--delete` flag ensures exact mirroring for folders
- File watcher includes 2-second cooldown to prevent sync loops
- All changes committed from workspace directory maintain Git history
