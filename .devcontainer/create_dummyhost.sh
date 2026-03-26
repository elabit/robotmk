#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CMK_ETC_DIR=/omd/sites/cmk/etc/check_mk
CMK_RULES_DIR=$WORKSPACE/.devcontainer/conf/checkmk

SECRETFILE=/opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret
if [[ ! -r "${SECRETFILE}" ]]; then
    echo "ERROR: In order to create a dummy host with this script, you must first create an automation user and store the secret in clear text!"
    exit 1
fi

# source cmk_version.sh to get CMK_VERSION_MM variable
source "${SCRIPT_DIR}/cmk_version.sh"


CMK_HOST="localhost"
SITE_NAME="cmk"
HOST="$(hostname)"
PROTO="http"
PORT=5000
API_URL="${PROTO}://${CMK_HOST}:${PORT}/${SITE_NAME}/check_mk/api/1.0"

USERNAME="automation"
PASSWORD="$(<"${SECRETFILE}")"

# verify that you are running as user cmk
if [ "$(id -u)" -ne 1000 ]; then
    echo "ERROR: This script must be run as the 'cmk' user inside the container. Current UID: $(id -u)"
    exit 1
fi

echo "Running as user: $(id -un) (UID: $(id -u))"
echo "cmk command is: $(which cmk)"
echo "CMK version: $(cmk --version | head -n1)"

echo "Automation password: ${PASSWORD}"
echo "+ Creating dummy host ${HOST} via API... "
if ! curl \
    --silent \
    --show-error \
    --request POST \
    --header "Authorization: Bearer ${USERNAME} ${PASSWORD}" \
    --header "Accept: application/json" \
    --header "Content-Type: application/json" \
    --data "{\"attributes\": {\"ipaddress\": \"127.0.0.1\"},\"folder\": \"/\",\"host_name\": \"${HOST}\"}" \
    "${API_URL}/domain-types/host_config/collections/all"; then
    echo "WARNING: Failed to create dummy host (it may already exist)." >&2
fi

echo "+ Reloading CMK config ... "
cmk -R

RULES_MK=/omd/sites/cmk/etc/check_mk/conf.d/wato/rules.mk

# if CMK version > 2.4, use rules25.mk.txt (different valuespecs), else use rules.mk.txt
if [[ "$CMK_VERSION_MM" == "2.5" ]]; then
    RULES_TPL="${CMK_RULES_DIR}/rules25.mk.txt"
else
    RULES_TPL="${CMK_RULES_DIR}/rules.mk.txt"
fi

if ! grep -q robotmk "${RULES_MK}" 2>/dev/null; then
    echo "+ Replacing hostname in ${RULES_TPL} and adding it to ${RULES_MK} ... "
    # Use sed instead of envsubst to replace $HOSTNAME variable
    sed "s/\$HOSTNAME/${HOST}/g" "${RULES_TPL}" >> "${RULES_MK}"
else
    echo "+ robotmk agent config already in rules.mk ... "
fi

echo "+ Adding ignore rules to $CMK_ETC_DIR/final.mk ... "
cat $CMK_RULES_DIR/final.mk.txt > $CMK_ETC_DIR/final.mk


echo "+ Discovering ... "
cmk -IIv >/dev/null 2>&1
echo "+ Reloading CMK config ... "
cmk -R
