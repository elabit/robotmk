#!/bin/bash

export VERSION=$1
export MAJOR_VERSION=${VERSION%%.*}


echo "+ Generating devcontainer file for CMK $VERSION..."
envsubst < .devcontainer/v$MAJOR_VERSION/devcontainer.json > .devcontainer/devcontainer.json
# devcontainer.json contains a VS Code Variable ${containerWorkspaceFolder}, which would also 
# be processed by envsubst. To avoid this, the template files contain ###{containerWorkspaceFolder}.
# The three hashes are replaced with $ _after_ envsusbt has done its work. 
# Mac-only sed... 
sed -i "" 's/###/$/' .devcontainer/devcontainer.json

echo "+ Configuring Python for CMK $MAJOR_VERSION... "
cat .vscode/v$MAJOR_VERSION/settings.json > .vscode/settings.json 

echo "+ Setting debug configuration for CMK $MAJOR_VERSION "
cat .vscode/v$MAJOR_VERSION/launch.json > .vscode/launch.json 
echo
echo ">>> Preparation for Checkmk version $VERSION finished."
echo "You can now start the devcontainer in VS Code with 'Remote-Containers: Rebuild Container'."