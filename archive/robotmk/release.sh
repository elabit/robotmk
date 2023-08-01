#!/bin/bash

PKG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

function usage() {
    echo "Usage: $0 <part> [<commit message>]"
    echo "  <commit message> - commit message for the release"
    exit 1
}

# check for two arguments

if [ $# -lt 1 ]; then
    usage
else
    part=$1
    msg=$2
fi

# check if bumpversion is installed
if ! command -v bumpversion &>/dev/null; then
    echo "bumpversion could not be found. Please install before. Exiting..."
    exit 1
fi

pushd "$PKG_ROOT" || exit

# check if there are any uncommitted changes
if [[ $(git status -s) ]]; then
    if [ "$msg" == "" ]; then
        echo "Git is dirty. Please provide a commit message. Exiting..."
        exit 1
    else
        git add .. && git commit -m "$msg"
    fi
else
    # git is clean
    if [ "$msg" != "" ]; then
        echo "A commmit message was provided, but git is clean already. Exiting..."
        exit 1
    else
        echo "Git is clean. Proceeding..."
    fi
fi

bumpversion $part --tag --commit &&
    git push &&
    git push --tags

flit publish

popd || exit
