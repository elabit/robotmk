#!/bin/bash

# This script fixes the permissions for all robotmk files which are symlinked into OMD. 
# (switching branches makes files write-protected for the site user)
# Works for CMK v1.6 and 2.x sites
# Usage: linkfiles.sh SITENAME

SCRIPT=$(readlink -f "$0")
SCRIPTPATH=$(dirname "$SCRIPT")
OMDSITES=/opt/omd/sites

function main {
    SITE=$1
    if [ "x$SITE" == "x" ]; then 
        usage
        exit 
    fi
    version=$(siteversion $SITE)
    if [[ $version =~ ^1.6 ]]; then 
        echo "Version 1"
        linkcommonfiles
        linkv1files
    elif [[ $version =~ ^2 ]]; then 
        echo "Version 2"
        linkcommonfiles
        linkv2files
    else
        echo "Version $version of site $SITE is not supported by this script. Exiting."
        usage
    fi 
}

function usage {
    echo "Usage: $0 SITENAME"
    exit 1
}

function linkcommonfiles {
    # Agent plugins
    relink agents/plugins/robotmk.py                         local/share/check_mk/agents/plugins/
    relink agents/plugins/robotmk-runner.py                  local/share/check_mk/agents/plugins/
    # Metrics
    relink web/plugins/metrics/robotmk.py                    local/share/check_mk/web/plugins/metrics/
    # WATO pages
    relink web/plugins/wato/robotmk_wato_params_bakery.py    local/share/check_mk/web/plugins/wato/
    relink web/plugins/wato/robotmk_wato_params_check.py     local/share/check_mk/web/plugins/wato/
    relink web/plugins/wato/robotmk_wato_params_discovery.py local/share/check_mk/web/plugins/wato/
}

function linkv1files {
    # Check plugin
    relink checks/robotmk                                    local/share/check_mk/checks/robotmk
    # bakery script
    relink agents/bakery/robotmk.py                             local/share/check_mk/agents/bakery/robotmk.py
}
function linkv2files {
    # Check plugin
    relink lib/check_mk/base/plugins/agent_based/robotmk.py  local/lib/check_mk/base/plugins/agent_based/robotmk.py
    # bakery script
    relink lib/check_mk/base/cee/plugins/bakery/robotmk.py   local/lib/check_mk/base/cee/plugins/bakery/robotmk.py
}

function siteversion {
    site=$1
    version=$(basename $(readlink -f /omd/sites/$site/version))
    echo $version
}

function relink {
    TARGET="$SCRIPTPATH/$1"
    LINKNAME="$OMDSITES/$SITE/$2"
    echo "Linking $TARGET into OMD site $SITE ..."
    ln -fs $TARGET $LINKNAME       
    chmod 666 $TARGET
}

main $@