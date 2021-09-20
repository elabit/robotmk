#!/bin/bash

VERSION=$1

echo "+ Applying CMK$VERSION specific devcontainer.json file.."
cat .devcontainer/v$VERSION/devcontainer.json > .devcontainer/devcontainer.json

echo "+ Configuring Python for CMK$VERSION... "
cat .vscode/v$VERSION/settings.json > .vscode/settings.json 

echo "+ Setting debug configuration for CMK$VERSION "
cat .vscode/v$VERSION/launch.json > .vscode/launch.json 

echo "Preparation for Checkmk version $VERSION finished."
echo "You can now start the devcontainer in VS Code with 'Remote-Containers: Rebuild Container'."