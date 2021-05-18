#!/bin/bash
set -e

# CMK Major version 
MVERSION="$(cat $OMD_ROOT/.version_meta/version | cut -d '.' -f1)"
NAME="robotmk"
PACKAGEFILE=$OMD_ROOT/var/check_mk/packages/$NAME

echo "HERE:----"
ls -la

# get the current tag (Release) or commit hash (Artifact)
export RMK_VERSION=$(git describe --exact-match --tags 2> /dev/null || git rev-parse --short HEAD)


rm -f $OMD_ROOT/var/check_mk/packages/*

echo "---------------------------------------------"
echo "* Merging the common package info with version $MVERSION specific..."
jq -s '.[0] * .[1]' $WORKSPACE/package/pkginfo_common $WORKSPACE/package/v$MVERSION/pkginfo | jq '. + {version:env.RMK_VERSION}' > $PACKAGEFILE
echo "---------------------------------------------"
echo "$PACKAGEFILE:"
cat $PACKAGEFILE
echo "---------------------------------------------"
echo "* Building MKP '$NAME' on $RMK_VERSION for CMK version $MVERSION..."
set -x
ls -la $PACKAGEFILE
mkp -v pack $NAME
FILE=$(ls -1 *.mkp)
# robotmk.cmk2-v1.1.0.mkp
# robotmk.v1.1.0-cmk2.mkp
NEWFILENAME=$NAME.$RMK_VERSION-cmk$MVERSION.mkp
mv $FILE $NEWFILENAME
echo "---------------------------------------------"

if [ -n "$GITHUB_WORKSPACE" ]; then
    echo "* Set Outputs for GitHub Workflow steps"
    echo "::set-output name=pkgfile::$NEWFILENAME"
    # echo "::set-output name=pkgname::${NAME}"
    VERSION=$(jq -r '.version' $PACKAGEFILE)
    # echo "::set-output name=pkgversion::$RMK_VERSION"
    # echo "::set-output name=cmkmversion::$MVERSION"
    echo "::set-output name=artifactname::$NEWFILENAME"
else 
    echo "* (No GitHub Workflow detected)"
fi
echo "END OF build.sh"
echo "---------------------------------------------"