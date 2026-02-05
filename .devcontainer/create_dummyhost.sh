#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CMK_VERSION_MM="${1:-}"
if [[ -z "${CMK_VERSION_MM}" ]]; then
    echo "Usage: $0 <cmk-version-mm>"
    exit 1
fi

SECRETFILE=/opt/omd/sites/cmk/var/check_mk/web/automation/automation.secret
if [[ ! -r "${SECRETFILE}" ]]; then
    echo "ERROR: In order to create a dummy host with this script, you must first create an automation user and store the secret in clear text!"
    exit 1
fi

CMK_HOST="localhost"
SITE_NAME="cmk"
HOST="${HOSTNAME:-dummyhost}"
PROTO="http"
PORT=5000
API_URL="${PROTO}://${CMK_HOST}:${PORT}/${SITE_NAME}/check_mk/api/1.0"

USERNAME="automation"
PASSWORD="$(<"${SECRETFILE}")"

echo "Automation password: ${PASSWORD}"
echo "+ Creating a dummy host via API... "
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
if ! grep -q robotmk "${RULES_MK}" 2>/dev/null; then
    echo "+ Adding rules.mk, replacing HOSTNAME with ${HOST} via envsubst ... "
    CFG="$(envsubst < "${REPO_ROOT}/.devcontainer/rules.mk.txt")"
    echo "${CFG}" >> "${RULES_MK}"
else
    echo "+ robotmk agent config already in rules.mk ... "
fi

case "${CMK_VERSION_MM}" in
    2.2)
        # 2.2 already reloaded; nothing else to do.
        ;;
    2.3|2.4|2.5)
        echo "+ Discovering ... "
        cmk -IIv >/dev/null 2>&1
        echo "+ Reloading CMK config ... "
        cmk -R
        ;;
    *)
        echo "WARNING: Unsupported CMK version '${CMK_VERSION_MM}'; skipping discovery." >&2
        ;;
esac