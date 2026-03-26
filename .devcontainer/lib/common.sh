#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# Common utility functions for Robotmk development and testing scripts
# Source this file in other scripts to access shared functionality
#set -x

# Print a formatted section header
function print_header() {
    local message="$1"
    echo "========================="
    echo "$message"
    echo "========================="
}

# Print a formatted sub-header
function print_subheader() {
    local message="$1"
    echo "-------------------------"
    echo "$message"
    echo "-------------------------"
}

# Assert that a command exists in PATH
# Usage: assert_command_exists docker "Docker is required"
function assert_command_exists() {
    local cmd="$1"
    local msg="${2:-Command '$cmd' is required but not found in PATH}"
    
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "ERROR: $msg" >&2
        return 1
    fi
    return 0
}

# Load OMD/CMK site profile with error handling
# Sets environment variables from the site's .profile
function load_cmk_profile() {
    local profile_path="${1:-/omd/sites/cmk/.profile}"
    
    if [ ! -f "$profile_path" ]; then
        echo "ERROR: Profile not found at $profile_path" >&2
        return 1
    fi
    
    set -a
    # shellcheck disable=SC1090
    source "$profile_path"
    set +a
    
    if [ -z "${OMD_SITE:-}" ]; then
        echo "ERROR: OMD_SITE not set after sourcing profile" >&2
        return 1
    fi
    
    return 0
}

# Determine workspace path (handles both local and CI environments)
function get_workspace_path() {
    if [ -n "${WORKSPACE:-}" ]; then
        echo "$WORKSPACE"
    elif [ -n "${GITHUB_WORKSPACE:-}" ]; then
        echo "$GITHUB_WORKSPACE"
    else
        # Try to detect from script location
        local script_dir
        script_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
        # Assume we're in a subdirectory of the workspace
        echo "$(cd "$script_dir/.." && pwd)"
    fi
}

# Validate CMK version format
# Supports formats like: 2.5.0-2026.03.05, 2.4.0p12, 2.3.0p45
function validate_cmk_version() {
    local version="$1"
    
    if [ -z "$version" ]; then
        echo "ERROR: CMK version cannot be empty" >&2
        return 1
    fi
    
    # Check for valid version pattern (flexible to support different formats)
    if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
        echo "ERROR: Invalid CMK version format: $version" >&2
        echo "Expected format: X.Y.Z or X.Y.ZpNN or X.Y.Z-YYYY.MM.DD" >&2
        return 1
    fi
    
    return 0
}

# Extract major.minor version from full version string
# E.g., "2.5.0-2026.03.05" -> "2.5", "2.4.0p12" -> "2.4"
function get_version_mm() {
    local version="$1"
    echo "$version" | cut -d. -f1-2
}

# Generic cleanup handler for trap
# Usage: Set CLEANUP_COMMAND variable before calling
function cleanup_on_exit() {
    local exit_code=$?
    
    if [ -n "${CLEANUP_COMMAND:-}" ]; then
        echo "Running cleanup..." >&2
        eval "$CLEANUP_COMMAND"
    fi
    
    exit $exit_code
}

# Load available CMK versions from devcontainer_img_versions.env
# Returns versions as array via stdout (one per line)
function load_available_versions() {
    local versions_file="${1:-.devcontainer/devcontainer_img_versions.env}"
    
    if [ ! -f "$versions_file" ]; then
        echo "ERROR: Versions file not found: $versions_file" >&2
        return 1
    fi
    
    # shellcheck disable=SC1090
    source "$versions_file"
    
    if [ -z "${CMKVERSIONS:-}" ]; then
        echo "ERROR: CMKVERSIONS not defined in $versions_file" >&2
        return 1
    fi
    
    # Output each version on a separate line
    printf '%s\n' "$CMKVERSIONS"
}

# Interactive version selection menu
# Returns selected version via stdout
function select_version_interactive() {
    local versions_file="${1:-.devcontainer/devcontainer_img_versions.env}"
    
    echo "No CMK version specified. Select a version:" >&2
    
    local -a versions=()
    while IFS= read -r line; do
        [ -n "$line" ] && versions+=("$line")
    done < <(load_available_versions "$versions_file")
    
    if [ ${#versions[@]} -eq 0 ]; then
        echo "ERROR: No versions available" >&2
        return 1
    fi
    
    select v in "${versions[@]}"; do
        if [ -n "$v" ]; then
            echo "$v"
            return 0
        else
            echo "Invalid selection. Try again." >&2
        fi
    done
}

# ============================================================================
# Docker Container Helper Functions
# ============================================================================

# Get the hostname from inside a Docker container
# Usage: get_container_hostname <container_name>
function get_container_hostname() {
    local container_name="$1"
    
    if [ -z "$container_name" ]; then
        echo "ERROR: Container name is required" >&2
        return 1
    fi
    
    docker exec "$container_name" hostname
}

# Execute command in container as cmk user with login shell
# Usage: exec_in_container_as_cmk <container_name> <command> [site_user]
function exec_in_container_as_cmk() {
    local container_name="$1"
    local command="$2"
    local site_user="${3:-cmk}"
    
    docker exec "$container_name" su - "$site_user" -c "$command"
}

# Execute command in container as root
# Usage: exec_in_container_as_root <container_name> <command>
function exec_in_container_as_root() {
    local container_name="$1"
    local command="$2"
    
    docker exec -u root "$container_name" bash -c "$command"
}

# Bake CMK agent for a hostname inside container
# Usage: bake_agent_in_container <container_name> [hostname]
# If hostname is omitted, uses container's hostname
function bake_agent_in_container() {
    local container_name="$1"
    local hostname="${2:-}"
    
    if [ -z "$hostname" ]; then
        hostname=$(get_container_hostname "$container_name")
    fi
    
    exec_in_container_as_cmk "$container_name" "cmk -Avf '${hostname}' 2>&1 | tail -10"
}

# Install CMK agent package inside container
# Usage: install_agent_in_container <container_name> <site_name> [hostname]
function install_agent_in_container() {
    local container_name="$1"
    local site_name="$2"
    local hostname="${3:-}"
    
    if [ -z "$hostname" ]; then
        hostname=$(get_container_hostname "$container_name")
    fi
    
    exec_in_container_as_root "$container_name" "
        set -e
        dpkg -i /omd/sites/${site_name}/var/check_mk/agents/linux_deb/references/${hostname} 2>&1 | grep -E '(Selecting|Setting up|agent)' || true
    "
}

# Start xinetd service in container
# Usage: start_xinetd_in_container <container_name>
function start_xinetd_in_container() {
    local container_name="$1"
    
    exec_in_container_as_root "$container_name" "
        # Kill any existing xinetd
        pkill xinetd 2>/dev/null || true
        # Start xinetd
        xinetd 2>/dev/null || nohup xinetd >/dev/null 2>&1 &
        sleep 2
    " || true
}

# Run service discovery in container
# Usage: discover_services_in_container <container_name> [hostname]
function discover_services_in_container() {
    local container_name="$1"
    local hostname="${2:-}"
    
    if [ -z "$hostname" ]; then
        hostname=$(get_container_hostname "$container_name")
    fi
    
    exec_in_container_as_cmk "$container_name" "cmk -IIv '${hostname}' 2>&1 | tail -20"
}

# Reload CMK configuration in container
# Usage: reload_cmk_config_in_container <container_name>
function reload_cmk_config_in_container() {
    local container_name="$1"
    
    exec_in_container_as_cmk "$container_name" "cmk -R" >/dev/null
}

# Restart CMK site in container
# Usage: restart_cmk_site_in_container <container_name> <site_name>
function restart_cmk_site_in_container() {
    local container_name="$1"
    local site_name="$2"
    
    exec_in_container_as_cmk "$container_name" "omd restart"
}


# Export functions for use in other scripts
export -f print_header
export -f print_subheader
export -f assert_command_exists
export -f load_cmk_profile
export -f get_workspace_path
export -f validate_cmk_version
export -f get_version_mm
export -f cleanup_on_exit
export -f load_available_versions
export -f select_version_interactive
export -f get_container_hostname
export -f exec_in_container_as_cmk
export -f exec_in_container_as_root
export -f bake_agent_in_container
export -f install_agent_in_container
export -f start_xinetd_in_container
export -f discover_services_in_container
export -f reload_cmk_config_in_container
export -f restart_cmk_site_in_container
