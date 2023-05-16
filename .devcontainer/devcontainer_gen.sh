#!/bin/bash
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later

# This file is used to generate the devcontainer.json file. It is called from the project 
# root directory. It sets the container name to the project name and Checkmk version to
# ARG1 = Checkmk version, e.g. 2.1.0p11

# You can also place a devcontainer_local.json file in the same folder. It gets 
# applied on top of the generated devcontainer.json file. This allows you to
# customize the devcontainer.json further.

export VERSION=$1
DEVC_FILE=".devcontainer/devcontainer.json"
DEVC_LOCAL_FILE=".devcontainer/devcontainer_local.json"
DEVC_TPL_FILE=".devcontainer/devcontainer_tpl.json"

# project.env contains some generic useful variables
source project.env
export CONTAINER_NAME=${PROJECT_NAME}-devc

function main() {
    # if version is unset, exit with error
    if [ -z "$VERSION" ]; then
        echo "No cmk version (arg1) specified. Choose one of the following:"
        PWD=$(folder_of $0)
        cat $PWD/devcontainer_img_versions.env
        exit 1
    fi
    

    echo "+ Generating CMK devcontainer file ..."
    # Ref LeP3qq
    envsubst < $DEVC_TPL_FILE > $DEVC_FILE.tmp
    # devcontainer.json contains a VS Code Variable ${containerWorkspaceFolder}, which would also 
    # be processed by envsubst. To avoid this, the template files contain ###{containerWorkspaceFolder}.
    # The three hashes are replaced with $ _after_ envsusbt has done its work. 
    # Mac-only sed... 
    sed -i 's/###/$/' $DEVC_FILE.tmp
    if [ -f $DEVC_LOCAL_FILE ]; then
        echo "+ Merging local devcontainer file for project $PROJECT_NAME ..."
        jq -s '.[0] * .[1]' $PWD/$DEVC_FILE.tmp $PWD/$DEVC_LOCAL_FILE > $PWD/$DEVC_FILE
        rm $PWD/$DEVC_FILE.tmp
    else
        mv $PWD/$DEVC_FILE.tmp $PWD/$DEVC_FILE
    fi
    
    echo ">>> $DEVC_FILE for Checkmk version $VERSION created."
    echo "Container will start with name: '$CONTAINER_NAME'"
}


function folder_of() {
  DIR="${1%/*}"
  (cd "$DIR" && echo "$(pwd -P)")
}

main $@