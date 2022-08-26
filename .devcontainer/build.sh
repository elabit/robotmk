#!/bin/bash
set -e
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This file creates a CMK MKP file for the determined CMK version (1/2).
# It leverages the "mkp" command from CMK, which reads a package description file
# (JSON). This JSON has similar keys and version specific ones.
# The similar keys are read from package/pkginfo_common.
# Version specific keys are merged from package/v_/pkginfo. 

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
# CMK Major version 
MVERSION="$(cat $OMD_ROOT/.version_meta/version | cut -d '.' -f1)"
NAME="robotmk"
PACKAGEFILE=$OMD_ROOT/var/check_mk/packages/$NAME

# get the current tag (Release) or commit hash (Artifact)
export RMK_VERSION=$(git describe --exact-match --tags 2> /dev/null || git rev-parse --short HEAD)

echo "▹ Removing old packages..."
rm -f $OMD_ROOT/var/check_mk/packages/*

echo "---------------------------------------------"
echo "▹ Merging the common package info with version $MVERSION specific..."
jq -s '.[0] * .[1]' $WORKSPACE/package/pkginfo_common $WORKSPACE/package/v$MVERSION/pkginfo | jq '. + {version:env.RMK_VERSION}' > $PACKAGEFILE
echo "---------------------------------------------"
echo "$PACKAGEFILE:"
cat $PACKAGEFILE
echo "---------------------------------------------"
echo "▹ Building MKP '$NAME' on $RMK_VERSION for CMK version $MVERSION..."
# set -x
ls -la $PACKAGEFILE
mkp -v pack $NAME
FILE=$(ls -rt1 *.mkp | tail -1)
# robotmk.cmk2-v1.1.0.mkp
# robotmk.v1.1.0-cmk2.mkp
NEWFILENAME=$NAME.$RMK_VERSION-cmk$MVERSION.mkp
mv $FILE $NEWFILENAME
echo "---------------------------------------------"
echo "📦  Package:"
echo "$NEWFILENAME"


if [ -n "${GITHUB_WORKSPACE-}" ]; then
    echo "▹ Set Outputs for GitHub Workflow steps"
    echo "::set-output name=pkgfile::$NEWFILENAME"
    # echo "::set-output name=pkgname::${NAME}"
    VERSION=$(jq -r '.version' $PACKAGEFILE)
    # echo "::set-output name=pkgversion::$RMK_VERSION"
    # echo "::set-output name=cmkmversion::$MVERSION"
    echo "::set-output name=artifactname::$NEWFILENAME"
else 
    echo "(No GitHub Workflow detected)"
fi
echo "END OF build.sh"
echo "---------------------------------------------"