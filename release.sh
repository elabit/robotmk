#!/bin/bash

function main (){
    MODE=$1
    TAG=$2

    if [ $(basename $(pwd)) != 'robotmk' ]; then 
        echo "ERROR: You seem to be not in the project root dir. Exiting."
        exit 1
    fi

    if [[ ! "$MODE" =~ release ]]; then 
        echo "ERROR: Param 1 must be either 'release' or 'unrelease'. Exiting."
        exit 1
    fi 

    if [ "x$TAG" == "x" ]; then 
        echo "ERROR: Param 2 must be a version name without 'v', e.g. 1.0.1.  Exiting."
        exit 1
    fi 
    if [ ! -x $(which chag) ]; then 
        echo "ERROR: chag not found."
        echo "-> https://github.com/mtdowling/chag"
        exit 1
    fi    
    export TAG
    export VTAG="v$TAG"
    export preVTAG="pre-$VTAG"
    if [ $MODE == 'release' ]; then 
        release
    elif [ $MODE == 'unrelease' ]; then
        unrelease
    fi
}


function release() {
    assert_gh_login
    assert_tag_unique $VTAG
    assert_branch "develop"
    assert_notdirty

    header "Setting pre-release tag $preVTAG ..."
    git tag $preVTAG
    header "Moving changelog entries from Unreleased to $TAG ..."
    chag update $TAG
    header "Committing: 'CHANGELOG: $VTAG'"
    git add . && git commit -m "CHANGELOG: $VTAG"

    header "Replacing Robotmk version in repository ..."
    grep -Hlr 'ROBOTMK_VERSION =' * | grep -v release | xargs sed -i '' -e "s/ROBOTMK_VERSION =.*/ROBOTMK_VERSION = '$VTAG'/"

    header "Committing: 'Version bump $VTAG'"
    git add . && git commit -m "Version bump: $VTAG"
    echo "Workflow result and artifacts are on https://github.com/simonmeggle/robotmk/actions/workflows/mkp-artifact.yml!"

    header "Merging develop into master..."
    git checkout master
    git merge develop --no-ff --no-edit --strategy-option theirs
    header "Create annotated git tag from Changelog entry ..."
    chag tag --addv
    header "Pushing ..."
    git push origin master
    git push origin $VTAG
}

function unrelease() {
    assert_gh_login
    assert_branch "develop"
    # assert_notdirty
    header "Changing to develop branch ..."
    git checkout develop
    header "Removing the release with tag $VTAG ..."
    gh release delete $VTAG -y
    header "Removing tags ..."
    git push origin :refs/tags/$VTAG 
    git tag -d $VTAG
    header "Removing tags ..."
    exit 
    header "Resetting the 'develop' branch to the tag $preVTAG ..."
    git reset --hard $preVTAG
    git tag -d $preVTAG 
}

function assert_branch {
    BRANCH="$(git rev-parse --abbrev-ref HEAD)"
    if [[ "$BRANCH" != $1 ]]; then
        echo "ERROR: You are not in branch '$1'. Exiting."
        exit 1
    fi
}

function assert_notdirty {
    if [ -n "$(git status --porcelain)" ]; then 
        echo "ERROR: The working area is dirty; please commit first! Exiting."
        exit 1
    fi
}

function header() {
    echo "========================="
    echo "$1"
}

function assert_gh_login() {
    gh auth status 2>&1 > /dev/null
    if [ $? -gt 0 ]; then 
        echo "ERROR: you do not seem to be logged in with gh CLI. Exiting."
        exit 1
    fi 
}

function assert_tag_unique(){
    git tag | egrep -q "^$1$"
    if [ $? -eq 0 ]; then 
        echo "ERROR: Tag $1 exists already. Exiting."
        exit 1
    fi
}


main $@
