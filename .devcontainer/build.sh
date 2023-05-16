#!/bin/bash
set -e
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This file creates a CMK MKP file for the determined CMK version (1/2).
# It leverages the "mkp" command from CMK, which reads a package description file
# (JSON).

# After the MKP has been built, the script check if it runs within a Github
# Workflow. If so, it sets the artifact name as output variable.

if [ -z $WORKSPACE ]; then
    echo "ERROR: WORKSPACE environment variable not set. Exiting."
    exit 1
fi

if [ -z $OMD_SITE ]; then
    echo "ERROR: You do not seem to be on a OMD site (variable OMD_SITE not set). Exiting."
    exit 1
fi

set -u
# CMK Major version - not needed anymore but lets keep this information as part of the package name
MVERSION="2"
NAME="robotmk"
PACKAGEFILE=$OMD_ROOT/var/check_mk/packages/$NAME

# Ownership can look dubious for git, fix this.
git config --global --add safe.directory $WORKSPACE
# get the current tag (Release) or commit hash (Artifact)
export RMK_VERSION=$(git describe --exact-match --tags 2>/dev/null || git rev-parse --short HEAD)

echo "▹ Removing old packages..."
rm -f $OMD_ROOT/var/check_mk/packages/*

echo "---------------------------------------------"
echo "▹ Generating package infofile ..."
jq '. += {version:env.RMK_VERSION}' $WORKSPACE/pkginfo >$PACKAGEFILE

echo "---------------------------------------------"
echo "$PACKAGEFILE:"
cat $PACKAGEFILE
echo "---------------------------------------------"
echo "▹ Building the MKP '$NAME' on $RMK_VERSION ..."
# set -x
ls -la $PACKAGEFILE
mkp -v pack $NAME
FILE=$(ls -rt1 *.mkp | tail -1)
# robotmk.cmk2-v1.1.0.mkp
NEWFILENAME=$NAME.$RMK_VERSION-cmk$MVERSION.mkp
mv $FILE $NEWFILENAME
PKG_PATH=$(readlink -f "$NEWFILENAME")
echo "📦   $PKG_PATH"
echo "---------------------------------------------"
echo ""
echo "Checking for Github Workflow..."
if [ -n "${GITHUB_WORKSPACE-}" ]; then
    echo "🐙 ...Github Workflow exists."
    echo "▹ Set Outputs for GitHub Workflow steps"
    echo "::set-output name=pkgfile::$NEWFILENAME"
    # echo "::set-output name=pkgname::${NAME}"
    VERSION=$(jq -r '.version' $PACKAGEFILE)
    # echo "::set-output name=pkgversion::$RMK_VERSION"
    # echo "::set-output name=cmkmversion::$MVERSION"
    echo "::set-output name=artifactname::$NEWFILENAME"
else
    echo "...no GitHub Workflow detected (local execution)."
fi
echo "END OF build.sh"
echo "---------------------------------------------"
