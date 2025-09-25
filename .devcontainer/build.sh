HEREIWAS: buildscript mit 2.2-pgkinfo laufen lassen

#!/bin/bash
set -e
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# This file creates a CMK MKP file.
# It leverages the "mkp" command from CMK, which reads a package description file
# (JSON).

# After the MKP has been built, the script checks if it runs within a Github
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
# Determine CMK major.minor for package naming (e.g., 2.2, 2.3, 2.4)
OMD_VER=$(omd version | awk '{print $NF}')
CMK_MM=$(echo "$OMD_VER" | cut -d. -f1-2)
NAME="robotmk"
PACKAGEFILE=$OMD_ROOT/var/check_mk/packages/$NAME
PKGDIR=$OMD_ROOT/var/check_mk/packages_local
PKG_DEST_DIR=$WORKSPACE/build


# Ownership can look dubious for git, fix this.
git config --global --add safe.directory $WORKSPACE

# If there is "## Unreleased" then assume a manual build, prompt the user to enter the version. 
# otherwise, take the version form the first H2 header in the CHANGELOG.md, format is: 
# ## 1.4.4 - 2024-03-26
# or 
# ## 1.4.4
# and extract the version number.

if [ -n "$(awk '/^## Unreleased/ {print $2; exit}' "$WORKSPACE/CHANGELOG.md")" ]; then
    echo "ERROR: Changelog contains Unreleased version. Please enter the version number manually."
    read -p "Enter the version number: " RMK_VERSION
else
    echo "Reading the first version from the CHANGELOG.md..."
    export RMK_VERSION=${RMK_VERSION:-$(awk '/^## [0-9]+\.[0-9]+/ {print $2; exit}' "$WORKSPACE/CHANGELOG.md")}
fi

echo "RMK_VERSION: $RMK_VERSION"


# both check and agent plugin need the same version number. String is to be replaced with the version number
# ROBOTMK_VERSION = 'x.x.x'
echo "Setting the version number $RMK_VERSION in the check and agent plugin..."
sed -i "s/ROBOTMK_VERSION =.*/ROBOTMK_VERSION = '$RMK_VERSION'/" $WORKSPACE/checks/robotmk.py
sed -i "s/ROBOTMK_VERSION =.*/ROBOTMK_VERSION = '$RMK_VERSION'/" $WORKSPACE/agents_plugins/robotmk.py


# TODO: why? 
#echo "▹ Removing old packages..."
#rm -f $OMD_ROOT/var/check_mk/packages/*

echo "---------------------------------------------"
PACKAGEFILE_TEMPLATE=$WORKSPACE/pkginfo/robotmk_cmk$CMK_MM.json
echo "▹ Generating package infofile using the $PACKAGEFILE template"


jq --arg version "$RMK_VERSION" \
   --arg version_packaged "$OMD_VER" \
   --arg version_min_required "${CMK_MM}.0p1" \
   --arg version_usable_until "${CMK_MM}.200" \
   '
   .version = $version
   | .["version.packaged"] = $version_packaged
   | .["version.min_required"] = $version_min_required
   | .["version.usable_until"] = $version_usable_until
   ' \
   "$PACKAGEFILE_TEMPLATE" > "$PACKAGEFILE"

echo "---------------------------------------------"
echo "$PACKAGEFILE:"
cat $PACKAGEFILE
echo "---------------------------------------------"

echo "▹ Building the MKP '$NAME' v$RMK_VERSION for CMK $CMK_MM ..."
# set -x
mkp -v package $PACKAGEFILE


FILE=$(ls -rt1 $PKGDIR/*.mkp | tail -1)
# robotmk-<ver>.mkp => rename to include cmk major.minor
NEWFILENAME=$NAME.$RMK_VERSION-cmk$CMK_MM.mkp
mkdir -p $PKG_DEST_DIR
mv $FILE $PKG_DEST_DIR/$NEWFILENAME
PKG_PATH=$PKG_DEST_DIR/$NEWFILENAME
echo "📦   $PKG_PATH"
echo "---------------------------------------------"

echo ""
echo "Checking for Github Workflow..."
if [ -n "${GITHUB_WORKSPACE-}" ]; then
    echo "🐙 ...Github Workflow exists."
    echo "▹ Set Outputs for GitHub Workflow steps"
    echo "::set-output name=pkgfile::$NEWFILENAME"
    # echo "::set-output name=pkgname::${NAME}"
    # dirty hack - won't spent more time into this...
    VERSION="$RMK_VERSION"
    # echo "::set-output name=pkgversion::$RMK_VERSION"
    # echo "::set-output name=cmkmversion::$MVERSION"
    echo "::set-output name=artifactname::$NEWFILENAME"
else
    echo "...no GitHub Workflow detected (local execution)."
fi
echo "END OF build.sh"
echo "---------------------------------------------"
