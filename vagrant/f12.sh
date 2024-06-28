#!/bin/bash

set -e

# Todo: Only update out-of-date things.

topdir="$(git rev-parse --show-toplevel)"

agentdir="C:/ProgramData/checkmk/agent"
agentplugindir="$agentdir"/plugins
agentconfigdir="$agentdir"/config
agentbindir="$agentdir"/bin
sshconfig="$topdir"/vagrant/ssh-config

# Store ssh-config
hostname=default
vagrant ssh-config > "$sshconfig"

# Sync rust binaries to vagrant machine (scheduler and agent plugin)
(cd "$topdir"; cargo build --example perm --target=x86_64-pc-windows-gnu)
sshpass -p "vagrant" scp -F "$sshconfig" "$topdir"/target/x86_64-pc-windows-gnu/debug/examples/perm.exe vagrant@"$hostname":"$agentbindir"
sshpass -p "vagrant" ssh -F "$sshconfig" "$hostname" "${agentbindir}/perm.exe"
# sshpass -p "vagrant" scp -F "$sshconfig" "$topdir"/target/x86_64-pc-windows-gnu/debug/robotmk_agent_plugin.exe vagrant@"$hostname":"$agentplugindir"

# Sync config for agent plugin to vagrant machine
# sshpass -p "vagrant" scp -F "$sshconfig" "$topdir"/data/retry_rcc/windows.json vagrant@"$hostname":"$agentconfigdir"/robotmk.json
