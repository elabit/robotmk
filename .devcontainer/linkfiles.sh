#!/bin/bash

# This script gets called from postcreateCommand.sh directly after the devcontainer
# has been started. Its job is to make the Robotmk project files available to the CMK site.
# For CMK v1 the directories get synchronized with lsyncd (the bakery does not 
# accept symlinks during the rpmbuild process). This concept will perhaps also 
# be chosen for V2.  
# lysncd gets installed within Dockerfile. 

LSYNCD_CFG=$OMD_ROOT/.lsyncd

function main {
    MVERSION=$(cat $OMD_ROOT/.version_meta/version | cut -d '.' -f1)
    echo -n "Site $OMD_SITE: "
    if [ $MVERSION == 1 ]; then 
        echo "Detected CMK major version 1"
        lsync_v1files
    elif [ $MVERSION == 2 ]; then 
        echo "Detected CMK major version 2"
        lsync_v2files
    else
        echo "Detected CMK major version $MVERSION is not supported by this script (only 1 and 2). Exiting."
        usage
    fi 
    echo -e "\n###########\nStarting lsyncd to synchronize files...\n"
    nohup lsyncd $OMD_ROOT/.lsyncd 2>&1 > /dev/null
}

function rmpath {
    echo "clearing $1"
    rm -rf $1
}

function linkpath {
    TARGET=$WORKSPACE/$1
    LINKNAME=$2
    echo "linking $TARGET -> $LINKNAME"
    # make sure that the link's parent dir exists
    mkdir -p $(dirname $LINKNAME)
    ln -sf $TARGET $LINKNAME
    #chmod 666 $TARGET/*
}

# Do not only symlink, but also generate needed directories. 
function link {
    echo "---"
    TARGET=$1
    if [ ${2:0:1} == "/" ]; then 
        LINKNAME=$2
    else
        LINKNAME=/omd/sites/cmk/$2
    fi    
    #rmpath $LINKNAME
    linkpath $TARGET $LINKNAME
}

function write_lsync_cfg {
    SOURCE=$1
    DEST=$2
    cat <<EOF >> $LSYNCD_CFG
sync {
  default.rsync,
  source = "$SOURCE",
  target = "$DEST",
  delay = 1,
}
EOF
}

# Synchronize a certain folder with lsyncd: 
# - do an initial rsync from the workspace into the dest dir
# - write the lsyncd config for a two-way-sync 
function lsync_this {
    echo "---"
    SOURCE=$WORKSPACE/$1
    
    if [ ${2:0:1} == "/" ]; then 
        DEST=$2
    else
        DEST=$OMD_ROOT/$2
    fi
    echo "Workspace dir: $SOURCE"
    echo "Container dir: $DEST"
    echo "> writing lsync config... "
    echo "-- $DEST" >> $LSYNCD_CFG
    mkdir -p $DEST
    rsync --quiet -ap $SOURCE/ $DEST
    write_lsync_cfg $SOURCE $DEST
    write_lsync_cfg $DEST $SOURCE
    echo >> $LSYNCD_CFG
    # uncomment this to debug the script and see if the initial sync was successful:
    #ls -la $DEST 
}

# Global lsyncd settings
function write_lsyncd_header {
    cat > $OMD_ROOT/.lsyncd << "EOF"
settings {
   inotifyMode = "CloseWrite or Modify"
}


EOF
}

function lsync_v1files {
    # write global lsyncd config
    write_lsyncd_header
    # Sync checkman
    lsync_this checkman local/share/check_mk/checkman
    # Sync Metrics, WATO
    lsync_this web_plugins local/share/check_mk/web/plugins
    # Sync agent_plugins
    lsync_this agents_plugins local/share/check_mk/agents/plugins
    # Sync agent plugins also as custom package 'robotmk-external' (which can be 
    # deployed with rule "deploy custom files". 
    # (the V2 bakery can handle this automatically for mode "external")
    lsync_this agents_plugins local/share/check_mk/agents/custom/robotmk-external/lib/bin
    # Sync check plugin dir
    lsync_this checks/v1 local/share/check_mk/checks
    # Sync Bakery script dir
    lsync_this bakery/v1 local/share/check_mk/agents/bakery
    # Images & icons
    lsync_this images local/share/check_mk/web/htdocs/images    
    # Sync RF test suites 
    lsync_this rf_tests /usr/lib/check_mk_agent/robot
    # Folder where agent output can be sourced with rule
    # "Datasource Programs > Individual program call instead of agent access"
    # (folder gets created in postCreateCommand.sh)
    lsync_this agent_output var/check_mk/agent_output
}

function lsync_v2files {
    # write global lsyncd config
    write_lsyncd_header    
    # checkman
    lsync_this checkman local/share/check_mk/checkman
    # Metrics, WATO, agent_plugins
    lsync_this web_plugins local/share/check_mk/web/plugins
    lsync_this agents_plugins local/share/check_mk/agents/plugins
    # Check plugin dir
    lsync_this checks/v2 local/lib/check_mk/base/plugins/agent_based
    # Bakery script dir
    lsync_this bakery/v2 local/lib/check_mk/base/cee/plugins/bakery
    # Images & icons
    lsync_this images local/share/check_mk/web/htdocs/images
    # # Sync RF test suites 
    lsync_this rf_tests /usr/lib/check_mk_agent/robot    
    # Folder where agent output can be sourced with rule
    # "Datasource Programs > Individual program call instead of agent access"
    # (folder gets created in postCreateCommand.sh)
    lsync_this agent_output var/check_mk/agent_output    
}

main
