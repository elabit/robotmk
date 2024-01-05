#!/bin/bash

set -e

# Todo: Only update out-of-date things.

topdir="$(git rev-parse --show-toplevel)"

agentdir="C:/ProgramData/checkmk/agent"
agentplugindir="$agentdir"/plugins
agentconfigdir="$agentdir"/config
agentbindir="$agentdir"/bin

# Sync rust binaries to vagrant machine (scheduler and agent plugin)
(cd "$topdir"; cargo build --target=x86_64-pc-windows-gnu)
sshpass -p "vagrant" scp -P 2222 "$topdir"/target/x86_64-pc-windows-gnu/debug/robotmk_scheduler.exe vagrant@127.0.0.1:"$agentbindir"
sshpass -p "vagrant" scp -P 2222 "$topdir"/target/x86_64-pc-windows-gnu/debug/robotmk_agent_plugin.exe vagrant@127.0.0.1:"$agentplugindir"

# Sync config for agent plugin to vagrant machine
sshpass -p "vagrant" scp -P 2222 "$topdir"/data/retry_rcc/windows.json vagrant@127.0.0.1:"$agentconfigdir"/robotmk.json
