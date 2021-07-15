#!/bin/bash

VERSION=$1

echo "+ Applying version specific devcontainer.json file.."
cp .devcontainer/devcontainer{_v$1,}.json

echo "+ Setting Python version for VS Code... "

if [ $VERSION -eq "1" ]; then 
    sed -i -e 's#cmk/bin/python3"#cmk/bin/python"#' .vscode/settings.json 
else 
    sed -i -e 's#cmk/bin/python"#cmk/bin/python3"#' .vscode/settings.json 
fi

echo "Preparation for Checkmk version $VERSION finished. You can now start the devcontainer in VS Code with 'Remote-Containers: Rebuild Container'."