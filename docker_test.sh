#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This script is used to test Robotmk in a Docker container.
# It has two modes:
# - link: Runs the container and links the files from the project to the CMK site
# - mkp: Runs the container and installs the MKP from build/ in the container
#
# In both cases, it then:
# - Links the rf_tests/ dir to /usr/lib/check_mk_agent/robot
# - Creates a dummyhost and installs the rule to monitor the test
# - Installs and configures the CMK agent
# - Discovers services
#
# The Checkmk user is always "cmk" and the site name is always "cmk".

#set -x

set -euo pipefail

# Script directory and workspace
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_WORKSPACE="${SCRIPT_DIR}"
# inside the container
WORKSPACE="/workspace/robotmk"

# Source common utilities
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/.devcontainer/lib/common.sh"

# Source project variables
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/project.env"



# ============================================================================
# Configuration & Constants
# ============================================================================

readonly SITE_NAME="cmk"
readonly SITE_USER="cmk"
readonly SITE_UID=1000
readonly SITE_GID=1000

# Port mappings (avoid conflicts with devcontainer on 5000)
readonly PORT_WEB_HOST=8081
readonly PORT_WEB_CONTAINER=5000
readonly PORT_SECONDARY_HOST=8001
readonly PORT_SECONDARY_CONTAINER=8000

# Default Docker registry and image
readonly DOCKER_REGISTRY="checkmk"
readonly DOCKER_IMAGE_PREFIX="cmk-python3-dev"

# ============================================================================
# Global Variables (Set by argument parsing)
# ============================================================================

CMK_VERSION=""
CMK_VERSION_MM=""
MODE=""
MKP_PATH=""
CONTAINER_NAME=""
DOCKER_IMAGE=""
VERBOSE=0

# ============================================================================
# Usage & Help
# ============================================================================

function usage() {
    cat <<EOF
Usage: $0 --mode {link|mkp} [OPTIONS]

Test Robotmk in a Docker container with two modes:
  - link: Mount workspace files directly into container (development mode)
  - mkp:  Install Robotmk MKP package into container (testing mode)

OPTIONS:
  --mode MODE           Required: Testing mode (link|mkp)
  --cmkversion VERSION  Optional: Checkmk version (e.g., 2.5.0-2026.03.05)
                        If omitted, interactive selection is offered
  --mkp PATH            Required for mkp mode: Path to MKP file
  -v, --verbose         Enable verbose output
  -h, --help            Show this help message

EXAMPLES:
  # Link mode with interactive version selection
  $0 --mode link

  # Link mode with specific version
  $0 --mode link --cmkversion 2.5.0-2026.03.05

  # MKP mode with specific version and package
  $0 --mode mkp --cmkversion 2.5.0-2026.03.05 --mkp build/robotmk.1.6.0-cmk2.5.mkp

  # MKP mode with interactive version selection
  $0 --mode mkp --mkp build/robotmk.1.6.0-cmk2.5.mkp

DESCRIPTION:
  This script starts a Checkmk Docker container and configures it for
  Robotmk testing. It handles container lifecycle, file syncing/installation,
  agent setup, and service discovery.

  IMPORTANT: This script includes an INTERACTIVE step where you must manually
  create an automation user via the Checkmk Web UI. This is required and cannot
  be automated.

  The container will be accessible at:
    Web UI: http://localhost:$PORT_WEB_HOST/$SITE_NAME
    User:   cmkadmin / cmk

EOF
}

# ============================================================================
# Argument Parsing
# ============================================================================

function parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --mode)
                MODE="$2"
                shift 2
                ;;
            --cmkversion)
                CMK_VERSION="$2"
                shift 2
                ;;
            --mkp)
                MKP_PATH="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=1
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                echo "ERROR: Unknown option: $1" >&2
                usage
                exit 1
                ;;
        esac
    done
    
    # Validate required arguments
    if [ -z "$MODE" ]; then
        echo "ERROR: --mode is required" >&2
        usage
        exit 1
    fi
    
    if [[ ! "$MODE" =~ ^(link|mkp)$ ]]; then
        echo "ERROR: --mode must be 'link' or 'mkp', got: $MODE" >&2
        usage
        exit 1
    fi
    
    # Interactive version selection if not provided
    if [ -z "$CMK_VERSION" ]; then
        CMK_VERSION=$(select_version_interactive "${SCRIPT_DIR}/.devcontainer/devcontainer_img_versions.env")
        echo "Selected version: $CMK_VERSION" >&2
    fi
    
    # Validate version format
    if ! validate_cmk_version "$CMK_VERSION"; then
        exit 1
    fi
    
    # Extract major.minor version
    CMK_VERSION_MM=$(get_version_mm "$CMK_VERSION")
    export CMK_VERSION_MM
    
    # Validate MKP mode requirements
    if [ "$MODE" = "mkp" ]; then
        if [ -z "$MKP_PATH" ]; then
            echo "ERROR: --mkp is required when mode is 'mkp'" >&2
            usage
            exit 1
        fi
        
        if [ ! -f "$MKP_PATH" ]; then
            echo "ERROR: MKP file not found: $MKP_PATH" >&2
            exit 1
        fi
        
        if [[ ! "$MKP_PATH" =~ \.mkp$ ]]; then
            echo "ERROR: MKP file must have .mkp extension: $MKP_PATH" >&2
            exit 1
        fi
    fi
    
    # Set derived variables
    CONTAINER_NAME="${PROJECT_NAME}-test-cmk${CMK_VERSION_MM}"
    DOCKER_IMAGE="${DOCKER_IMAGE_PREFIX}:${CMK_VERSION}"
    
    export MODE CMK_VERSION CMK_VERSION_MM MKP_PATH CONTAINER_NAME DOCKER_IMAGE
}

# ============================================================================
# Logging
# ============================================================================

function log_info() {
    echo "[INFO] $*"
}

function log_verbose() {
    if [ $VERBOSE -eq 1 ]; then
        echo "[VERBOSE] $*"
    fi
}

function log_error() {
    echo "[ERROR] $*" >&2
}

# ============================================================================
# Container Lifecycle Management
# ============================================================================

function start_cmk_container() {
    print_header "Starting Checkmk Container"
    
    # Check if container already exists
    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}\$"; then
        log_info "Container $CONTAINER_NAME already exists. Removing..."
        docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
    fi
    
    log_info "Starting container: $CONTAINER_NAME"
    log_info "Docker image: $DOCKER_IMAGE"
    log_info "Mode: $MODE"
    
    # Prepare docker run arguments
    local -a docker_args=(
        "-dit"
        "--name" "$CONTAINER_NAME"
        "-p" "${PORT_WEB_HOST}:${PORT_WEB_CONTAINER}"
        "-p" "${PORT_SECONDARY_HOST}:${PORT_SECONDARY_CONTAINER}"
        "--tmpfs" "/opt/omd/sites/${SITE_NAME}/tmp:uid=${SITE_UID},gid=${SITE_GID}"
        "-v" "/etc/localtime:/etc/localtime:ro"
    )
    
    # Mount workspace based on mode
    if [ "$MODE" = "link" ]; then
        # Read-write mount for link mode (development)
        docker_args+=("-v" "${LOCAL_WORKSPACE}:${WORKSPACE}:rw")
        log_info "Mounting workspace (read-write): ${LOCAL_WORKSPACE} -> /workspace"
    else
        # Read-only mount for mkp mode (testing)
        docker_args+=("-v" "${LOCAL_WORKSPACE}:${WORKSPACE}:ro")
        log_info "Mounting workspace (read-only): ${LOCAL_WORKSPACE} -> /workspace"
    fi
    
    # Always mount rf_tests to the robot directory
    docker_args+=("-v" "${LOCAL_WORKSPACE}/rf_tests:/usr/lib/check_mk_agent/robot:ro")
    log_info "Mounting test directory: ${LOCAL_WORKSPACE}/rf_tests -> /usr/lib/check_mk_agent/robot"
    
    # Set environment variables
    docker_args+=("-e" "WORKSPACE=${WORKSPACE}")
    docker_args+=("-e" "CMK_VERSION_MM=${CMK_VERSION_MM}")
    docker_args+=("-e" "CMK_PASSWORD=cmk")
    
    # Add the image as the last argument
    docker_args+=("$DOCKER_IMAGE")
    
    # Start the container
    log_verbose "Docker command: docker run ${docker_args[*]}"
    
    if ! docker run "${docker_args[@]}" >/dev/null; then
        log_error "Failed to start container"
        return 1
    fi
    
    log_info "Container started successfully"
    
    # Wait for CMK to be ready
    wait_for_cmk_ready
}

function wait_for_cmk_ready() {
    print_subheader "Waiting for Checkmk to be ready"
    
    local max_attempts=60
    local attempt=0
    local wait_seconds=5
    
    log_info "Waiting for OMD site to start (max ${max_attempts} attempts, ${wait_seconds}s interval)..."
    
    while [ $attempt -lt $max_attempts ]; do
        attempt=$((attempt + 1))
        
        # Check if omd status shows the site as running
        if docker exec "$CONTAINER_NAME" su - "$SITE_USER" -c "omd status" 2>/dev/null | grep -q "Overall state:.*running"; then
            log_info "Checkmk is ready (attempt $attempt/$max_attempts)"
            return 0
        fi
        
        log_verbose "Attempt $attempt/$max_attempts: Checkmk not ready yet..."
        sleep $wait_seconds
    done
    
    log_error "Checkmk did not become ready after $max_attempts attempts"
    log_error "Container logs:"
    docker logs --tail 50 "$CONTAINER_NAME" >&2
    return 1
}

function stop_and_remove_container() {
    if [ -n "${CONTAINER_NAME:-}" ]; then
        print_subheader "Cleaning up container: $CONTAINER_NAME"
        
        if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}\$"; then
            log_info "Stopping and removing container..."
            docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
            log_info "Container removed"
        fi
    fi
}

# ============================================================================
# Link Mode Implementation
# ============================================================================

function setup_link_mode() {
    print_header "Setting up Link Mode"
    
    log_info "Syncing workspace files into container..."
    
    # Execute linkfiles.sh inside the container
    if ! docker exec -u "$SITE_USER" "$CONTAINER_NAME" bash -c "
        export WORKSPACE=${WORKSPACE}
        export CMK_VERSION_MM='${CMK_VERSION_MM}'
        source ${WORKSPACE}/.devcontainer/cmk_version.sh
        ${WORKSPACE}/.devcontainer/linkfiles.sh
    "; then
        log_error "Failed to sync files"
        return 1
    fi
    
    log_info "Files synced successfully"
    
    # Verify critical paths exist
    verify_robotmk_installation
}

function verify_robotmk_installation() {
    print_subheader "Verifying Robotmk installation"
    
    # Check for critical files in container
    log_info "Checking for Robotmk agent plugins in container..."
    
    if docker exec "$CONTAINER_NAME" bash -c "
        find /omd/sites/${SITE_NAME}/local/share/check_mk/agents/plugins -name 'robotmk*.py' | grep -q .
    "; then
        log_verbose "✓ Robotmk agent plugins found"
    else
        log_error "✗ Robotmk agent plugins not found"
        return 1
    fi
    
    log_info "Robotmk installation verified"
}

# ============================================================================
# MKP Mode Implementation
# ============================================================================

function setup_mkp_mode() {
    print_header "Setting up MKP Mode"
    
    local mkp_basename
    mkp_basename=$(basename "$MKP_PATH")
    local mkp_container_path="/tmp/${mkp_basename}"
    
    log_info "Copying MKP file into container..."
    log_info "Source: $MKP_PATH"
    log_info "Destination: $mkp_container_path"
    
    if ! docker cp "$MKP_PATH" "${CONTAINER_NAME}:${mkp_container_path}"; then
        log_error "Failed to copy MKP file to container"
        return 1
    fi
    
    log_info "Installing MKP package..."
    
    # Install and enable the MKP
    if ! docker exec "$CONTAINER_NAME" su - "$SITE_USER" -c "
        mkp add '${mkp_container_path}' && \
        mkp enable robotmk
    "; then
        log_error "Failed to install MKP package"
        return 1
    fi
    
    log_info "MKP package installed successfully"
    
    # Verify installation
    if ! docker exec "$CONTAINER_NAME" su - "$SITE_USER" -c "mkp list | grep -q robotmk"; then
        log_error "Robotmk not found in MKP list"
        return 1
    fi
    
    log_info "Robotmk package verified in MKP list"
    
    # Clean up temporary file
    docker exec "$CONTAINER_NAME" rm -f "$mkp_container_path" || true
}

# ============================================================================
# Test Setup & Configuration
# ============================================================================

function create_automation_user_interactive() {
    print_header "Create Automation User (Interactive)"
    
    cat <<EOF

╔════════════════════════════════════════════════════════════════════════════╗
║                   MANUAL STEP REQUIRED                                     ║
╚════════════════════════════════════════════════════════════════════════════╝

You need to create an automation user in the Checkmk Web UI:

  1. Open:  http://localhost:${PORT_WEB_HOST}/${SITE_NAME}/
     Login: cmkadmin / cmk

  2. Go to: Setup → Users → Add user

  3. If not exists, create user with these settings:
     Username:        automation
     Full name:       automation
     Authentication:  Automation secret for machine accounts
     Password:       (set any password, e.g., "automation")
     Store in clear text: yes (only in CMK 2.5+)
     Role:            Administrator
     

  4. Set the automation secret to: automation
     (or any password you prefer)

  5. IMPORTANT: Store the secret in clear text!
     (In the "Automation secret" field, ensure it's stored as plain text)

  6. Save the user

After creating the user, press ENTER to continue...

EOF
    
    read -p "Press ENTER once you have created the automation user: " -r
    echo
    log_info "Continuing with setup..."
}

function create_dummy_host() {
    print_header "Creating Test Host"
    
    # Execute create_dummyhost.sh inside the container as cmk user with login shell
    # HOSTNAME is the Checkmk monitored host name.

    log_info "Creating dummy host via CMK API..."
    
    if ! docker exec "$CONTAINER_NAME" su - "$SITE_USER" -c "
        export WORKSPACE=${WORKSPACE}
        cd ${WORKSPACE}
        ${WORKSPACE}/.devcontainer/create_dummyhost.sh '${CMK_VERSION_MM}'
    "; then
        log_error "Failed to create dummy host"
        log_error "This usually means the automation user was not created correctly."
        log_error "Please verify you created the automation user with secret stored in clear text."
        return 1
    fi
    
    log_info "Dummy host created successfully"
}

function install_and_configure_agent() {
    print_header "Installing Checkmk Agent"
    
    local cmk_hostname
    cmk_hostname=$(get_container_hostname "$CONTAINER_NAME")
    
    log_info "Baking agent package for host: ${cmk_hostname}..."
    if ! bake_agent_in_container "$CONTAINER_NAME" "$cmk_hostname"; then
        log_error "Failed to bake agent"
        return 1
    fi
    
    log_info "Installing agent package (running as root)..."
    if ! install_agent_in_container "$CONTAINER_NAME" "$SITE_NAME" "$cmk_hostname"; then
        log_error "Failed to install agent"
        return 1
    fi
    

    #log_info "Starting xinetd service..."
    #start_xinetd_in_container "$CONTAINER_NAME" || log_verbose "xinetd start completed"
    
    log_info "Agent installed and configured"
}

function discover_services() {
    print_header "Discovering Services"
    
    local cmk_hostname
    cmk_hostname=$(get_container_hostname "$CONTAINER_NAME")
    
    log_info "Running service discovery for host: ${cmk_hostname}..."
    
    if ! discover_services_in_container "$CONTAINER_NAME" "$cmk_hostname"; then
        log_error "Service discovery failed"
        return 1
    fi
    
    log_info "Reloading Checkmk configuration..."
    reload_cmk_config_in_container "$CONTAINER_NAME"
    
    log_info "Services discovered successfully"
}

# ============================================================================
# Main Workflow
# ============================================================================

function display_access_info() {
    print_header "Container Ready"
    cmk_hostname=$(get_container_hostname "$CONTAINER_NAME")
    
    cat <<EOF

✅ Robotmk test container is ready!

Container Information:
  Name:        $CONTAINER_NAME
  CMK Version: $CMK_VERSION
  Mode:        $MODE

Access Information:
  Web UI:      http://localhost:${PORT_WEB_HOST}/${SITE_NAME}/
  Username:    cmkadmin
  Password:    cmk
  
  Automation:  automation / automation

Test Host:
  Hostname:    ${cmk_hostname} (monitored in Checkmk)

Useful Commands:
  # View container logs
  docker logs -f $CONTAINER_NAME
  
  # Execute commands in container
  docker exec -it $CONTAINER_NAME su - cmk
  docker rm -f $CONTAINER_NAME

EOF
}

function main() {
    print_header "Robotmk Docker Test - $PROJECT_NAME"
    
    # Parse command line arguments
    parse_arguments "$@"
    
    # Verify prerequisites
    assert_command_exists docker "Docker is required to run this script"
    
    # TODO Enable: Set up cleanup trap
    CLEANUP_COMMAND="stop_and_remove_container"
    #trap cleanup_on_exit EXIT INT TERM
    
    # Start the container
    start_cmk_container
    
    # Mode-specific setup
    if [ "$MODE" = "link" ]; then
        setup_link_mode
    else
        setup_mkp_mode
    fi
    
    # Common setup steps (interactive automation user creation required)
    create_automation_user_interactive
    create_dummy_host
    install_and_configure_agent

    restart_cmk_site_in_container "$CONTAINER_NAME" "$SITE_NAME"
    discover_services
    
    # Display access information
    display_access_info
    
    # Remove cleanup trap (keep container running)
    trap - EXIT INT TERM
    
    log_info "Setup complete. Container is running and will remain active."
    log_info "Use 'docker rm -f $CONTAINER_NAME' to remove it when done."
}

# ============================================================================
# Script Entry Point
# ============================================================================

main "$@"


