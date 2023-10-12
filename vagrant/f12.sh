#!/bin/bash

set -e

# Todo: Only update out-of-date things.

topdir="$(git rev-parse --show-toplevel)"
rustdir="$topdir"/v2/robotmk/

agentdir="C:/ProgramData/checkmk/agent"
agentplugindir="$agentdir"/plugins
agentconfigdir="$agentdir"/config
agentbindir="$agentdir"/bin

# Sync robotmk.exe to vagrant machine
(cd "$rustdir"; cargo build --target=x86_64-pc-windows-gnu)
sshpass -p "vagrant" scp -P 2222 "$rustdir"/target/x86_64-pc-windows-gnu/debug/robotmk.exe vagrant@127.0.0.1:"$agentbindir"

# Sync collector.ps1 to vagrant machine
sshpass -p "vagrant" scp -P 2222 "$topdir"/checkmk_extensions/agent/robotmk_collector.ps1 vagrant@127.0.0.1:"$agentplugindir"

# Sync config for collector.ps1 to vagrant machine
sshpass -p "vagrant" scp -P 2222 "$topdir"/v2/data/retry_rcc/windows.json vagrant@127.0.0.1:"$agentconfigdir"/robotmk.json
