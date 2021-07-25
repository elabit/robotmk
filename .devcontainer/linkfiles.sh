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
        echo -e "\n###########\nStarting lsyncd to synchronize files...\n"
        nohup lsyncd ~/.lsyncd
    elif [ $MVERSION == 2 ]; then 
        echo "Detected CMK major version 2"
        linkv2files
    else
        echo "Detected CMK major version $MVERSION is not supported by this script (only 1 and 2). Exiting."
        usage
    fi 
}

function rmpath {
    echo "clearing $LINKNAME"
    rm -rf $OMD_ROOT/$1
}

function linkpath {
    TARGET=$WORKSPACE/$1
    LINKNAME=$OMD_ROOT/$2
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
    LINKNAME=$2
    rmpath $LINKNAME
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
    rsync --quiet -a $SOURCE/ $DEST
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
    # Sync RF test suites 
    lsync_this rf_tests /usr/lib/check_mk_agent/robot
}

function linkv2files {
    # checkman
    link checkman local/share/check_mk/checkman
    # Metrics, WATO, agent_plugins
    link web_plugins local/share/check_mk/web/plugins
    link agents_plugins local/share/check_mk/agents/plugins
    # Check plugin dir
    link checks/v2 local/lib/check_mk/base/plugins/agent_based
    # Bakery script dir
    link bakery/v2 local/lib/check_mk/base/cee/plugins/bakery
}

main