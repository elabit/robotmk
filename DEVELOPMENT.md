# Development

## Preconditions

### Development preconditions

#### VS Code

### Release preconditions

In addition to the above requirements, the following are needed to make new releases of Robotmk: 

#### Chag

(only for maintainer)
Robotmk uses [chag](https://raw.githubusercontent.com/mtdowling/chag/master/install.sh) to keep annotated tags and the CHANGELOG in sync. 

All unreleased work is documented under the `H2` "Unreleased": 

    ## Unreleased

    This will be the release title 

    
  * Show entries of a special release: `chag contents --tag v1.0.2`
  * Create a Changelog entry for the `Unreleased` section: `chag update 1.0.4`
  * Create an annotated tag from the 

#### Github CLI tool 

Authentication to GitHub is required if you want to release/unrelease.

### Others

#### Changelog

Robotmk's [CHANGELOG.md](CHANGELOG.md) is based on [](https://keepachangelog.com/).


## Environment setup 

### Starting the VS Code devcontainer

Both flavours of Robotmk (CMK1 and 2) can be developed with Visual Studio Code and the [devcontainer setup](https://code.visualstudio.com/docs/remote/containers). 

Before firing up the devcontainer, you have to select the CMK major version you want to develop in. 
- Run Cmd-Shift-P and select `Select Task...` and chose e.g. "Set devcontainer to CMKv2".
- Run Cmd-Shift-P and select `Run Task...` to run the task. This reconfigures some important files:

```
# Setting the environment for Checkmk major version 1: 
15:18 $ .devcontainer/set_devcontainer_version.sh 1
+ Applying version specific devcontainer.json file..
+ Setting Python version for VS Code...
Preparation for Checkmk version 1 finished. You can now start the devcontainer in VS Code with 'Remote-Containers: Rebuild Container'.
```

- Run Cmd-Shift-P and select `Remote-Containers: Rebuild Container` to start the devcontainer. 

In the VS Code terminal you see the CMK site starting. This takes some minutes (at least on my aged Mac). During this step, all relevant files for Robotmk get [lsynced](https://axkibe.github.io/lsyncd/) (V1)/symlinked (V1) into the version specific folder of the CMK Docker container. **Don't try to install the Robotmk MKP into this container!** 

If lsyncd is not running, do this by hand: `lsyncd .lsyncd`

The devcontainer is ready now.

### Select Python Interpreter 

After the devcontainer has started, you probably have to set the python interpreter explicitly. This is sometimes a little bit unreliable and must be done manually: 
- ensure that settings.json do not contain a `python.pythonPath` setting anymore
- Open Cmd-Shift-P and run "Select Python Interpreter" for the "robotmk" workspace




### VS Code Build Task

`Ctrl+Shift+B` is bound to `build.sh` which builds the CMK version specific MKP file. 

The resulting MKP can be copied to the host system as follows: 

```
CONTAINER=a596f322c2e8
cd ~/Downloads
docker exec $CONTAINER bash -c "mkdir -p /cmk-mkp; cp /workspaces/robotmk/*.mkp /cmk-mkp"
docker cp $CONTAINER:/cmk-mkp .
```


## Debugging

### Simulating agent output 

Agent output (e.g. from a CMK crash dump) can be injected into the container by placing the output as a file in the folder `agent_output`.

Then create a rule `Individual program call instead of agent access` which uses one of the following commands to source the output file instead of using an agent: 

    	cat ~/var/check_mk/agent_output/$HOSTNAME$
     cat ~/var/check_mk/agent_output/agent_output

### ipdb 

`ipdb` is a great cmdline debugger for Python. In the following example it is shown how to execute the Robotmk check within the cmk context. 
A breakpoint in line 120 is set with "b":  

Debugging the Inventory function:

```
OMD[cmk]:~$ python -m ipdb bin/cmk -IIv test2Win10simdows
> /opt/omd/sites/cmk/bin/cmk(34)<module>()
     33
---> 34 import os
     35 import sys

ipdb> b /omd/sites/cmk/local/share/check_mk/checks/robotmk:120
Breakpoint 1 at /omd/sites/cmk/local/share/check_mk/checks/robotmk:120
ipdb> r
Discovering services on: test2Win10simdows
test2Win10simdows:
+ FETCHING DATA
 [agent] Execute data source
 [piggyback] Execute data source
No piggyback files for 'test2Win10simdows'. Skip processing.
No piggyback files for '192.168.116.8'. Skip processing.
+ EXECUTING DISCOVERY PLUGINS (1751)
ps.perf does not support discovery. Skipping it.
> /omd/sites/cmk/local/share/check_mk/checks/robotmk(120)inventory_robot()
    119 def inventory_robot(robot_items):
1-> 120     robot_service_prefix = get_setting('robot_service_prefix',[])
    121     for robot_item in robot_items:
```

Debugging the bakery: 

```
OMD[v1test]:~$ python -m ipdb bin/cmk -Avf win10simdows
> /opt/omd/sites/v1test/bin/cmk(34)<module>()
     33
---> 34 import os
     35 import sys
ipdb> b /omd/sites/v1test/lib/python/cmk_base/cee/agent_bakery.py:85     
```

# How to release

`release.sh` is a helper tool which eases (un)releasing a lot. Sometimes a alpha/beta release should to be withdrawn. With the help of this script and the github CLI tool (authentication required),

## Release

The release workflow of Robotmk is divided into the following steps: 

* Make sure that the `develop` branch is clean (=everything is stashed/committed)
* Execute `./release.sh release 1.2.0`, which 
  * executes `chag update` => converting unreleased entries in `CHANGELOG` to the new version
  * replaces version number variables in Robotmk script files
  * commits this change as version bump 
  * merges `develop` into `master`
  * executes `chag tag --addv` => adds an annotated tag from the Changelog
  * pushes to `master`

## Unrelease

* Execute `./release.sh unrelease 1.2.0`, which 
* the release gets deleted from github 
* tags are removed
* develop branch gets checked out
* `chag` undoes the last change to the `CHANGELOG`
