#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later

# This script should be executed at the very beginning to craft Docker images based on
# the original Checkmk 1/2 Docker images which also contain Python 3.9 and Robotframework.
#
# 1) Edit build-devcontainer.env and change the variable CMKVERSIONS to your needs.
#    It should only contain CMK versions you want to test/develop on.
# 2) Start build-devcontainer.sh. It will check if the CMK Docker images are already
#    available locally. If not, it asks for credentials to download the
#    image from the CMK download page.
# 3) After the image tgz has been downloaded, it will be imported into Docker.
#    (approx. 5 minutes)
# 4) In the last step, the script will build an image based on the CMK version, including
#    Python3 and robotframework. (approx. 10 minutes)
# $ docker images | grep mk
# $CMK_PY3_DEV_IMAGE                                                2.0.0p5        1d96bebf47a6   27 seconds ago   2.18GB
# $CMK_PY3_DEV_IMAGE                                                1.6.0p25       599e8beeb9c7   10 minutes ago   1.93GB

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR

VERSION_FILE="${SCRIPT_DIR}/devcontainer_img_versions.env"
readonly VERSION_FILE

CMK_PY3_DEV_IMAGE="${CMK_PY3_DEV_IMAGE:-cmk-python3-dev}"
readonly CMK_PY3_DEV_IMAGE

DOCKERFILE_CMK_PY3_DEV="${DOCKERFILE_CMK_PY3_DEV:-Dockerfile_cmk_py3_dev}"
readonly DOCKERFILE_CMK_PY3_DEV

REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly REPO_ROOT

declare -a ALL_VERSIONS=()

main() {
    check_prerequisites
    load_versions
    build_images
}

check_prerequisites() {
    if ! command -v docker >/dev/null 2>&1; then
        printf 'docker command not found in PATH.\n' >&2
        exit 1
    fi
}

load_versions() {
    if [[ ! -f "${VERSION_FILE}" ]]; then
        printf 'Configuration file %s not found.\n' "${VERSION_FILE}" >&2
        exit 1
    fi

    # shellcheck source=.devcontainer/devcontainer_img_versions.env
    source "${VERSION_FILE}"

    if [[ -z "${CMKVERSIONS:-}" ]]; then
        printf 'CMKVERSIONS is not defined in %s.\n' "${VERSION_FILE}" >&2
        exit 1
    fi

    mapfile -t ALL_VERSIONS < <(printf '%s\n' "${CMKVERSIONS}")
}

image_exists() {
    local image_name=$1
    docker image inspect "${image_name}" >/dev/null 2>&1
}

prompt_download() {
    local image_name=$1
    local reply=""
    if ! read -r -p "Download ${image_name}? [y/N]: " reply; then
        return 1
    fi
    [[ ${reply} =~ ^[Yy]$ ]]
}

build_images() {
    export DOCKER_BUILDKIT=0
    local version
    for version in "${ALL_VERSIONS[@]}"; do
        [[ -z "${version}" ]] && continue
        build_single_image "${version}"
        echo ----
        echo ""
    done
}

build_single_image() {
    local version=$1
    local image_name="checkmk/check-mk-cloud:${version}"
    local target_image="${CMK_PY3_DEV_IMAGE}:${version}"

    if image_exists "${image_name}"; then
        printf 'Docker image %s is already available locally.\n' "${image_name}"
    else
        printf 'Docker image %s is not yet available locally.\n' "${image_name}"
        if prompt_download "${image_name}"; then
            if ! docker pull "${image_name}"; then
                printf '[ERROR] Download failed for %s.\n' "${image_name}" >&2
                exit 1
            fi
            printf 'Downloaded %s.\n' "${image_name}"
        else
            printf 'Skipping image build for Checkmk version %s.\n' "${version}"
            return
        fi
    fi

    printf 'Building local image %s from %s...\n' "${target_image}" "${DOCKERFILE_CMK_PY3_DEV}"
    printf 'Calling: docker build -t %s -f %s/%s --build-arg VARIANT=%s %s\n' \
        "${target_image}" "${SCRIPT_DIR}" "${DOCKERFILE_CMK_PY3_DEV}" "${version}" "${REPO_ROOT}"

    if ! DOCKER_BUILDKIT=0 docker build \
        -t "${target_image}" \
        -f "${SCRIPT_DIR}/${DOCKERFILE_CMK_PY3_DEV}" \
        --build-arg "VARIANT=${version}" \
        "${REPO_ROOT}"; then
        printf '[ERROR] Docker image %s could not be built.\n' "${target_image}" >&2
        exit 1
    fi

    printf 'Docker image %s has been built.\n' "${target_image}"
}

main "$@"
