#!/bin/bash

# This step ties the workspace files with the Devcontainer.
# V1 sites use lsyncd instead of symlinks because the bakery/rpmbuild needs real files instead of links. 
.devcontainer/linkfiles.sh
# Fire up the site
omd start