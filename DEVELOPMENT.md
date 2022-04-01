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

Both flavours of Robotmk (CMK1 and 2) can be developed with Visual Studio Code and the [devcontainer setup](https://code.visualstudio.com/docs/remote/containers). 



### Build Devcontainer 

Open `.devcontainer/build-devcontainer.env` and add all CMKVERSIONS to your needs. It should only contain CMK versions you want to test/develop on.
(All versions are a long quoted string, separated by newlines.)

     CMKVERSIONS="1.6.0p25
     2.0.0p5
     2.0.0p22
     2.1.0b4"

- Run Cmd-Shift-P and select `Select Task...` > "Build all devcontainer images".
- Run Cmd-Shift-P and select `Run Task...` to built the containers. 

What does the task do? 

- Check if the CMK Docker images are already available locally. If not, it asks for credentials to download the image from the CMK download page. 
- It creates a new Docker image from `Dockerfile_cmk_python`. This uses the CMK docker image as base and 
  - installs Python 3.9 
  - Python modules `robotframework pyyaml mergedeep python-dateutil ipdb`
  - and some additional tools: `jq tree htop vim git telnet file lsyncd`

The resulting image is saved as `robotmk-cmk-python3:2.1.0b4` (example) and is the base image for `Dockerfile` (which is referenced in `devcontainer.json`)

     16:04 $ docker images | grep robotmk-cmk
     robotmk-cmk-python3                            2.1.0b4        d1c5971438c3   About a minute ago   2.39GB
     robotmk-cmk-python3                            2.0.0p22       a9c63d994a74   9 minutes ago        2.19GB
     robotmk-cmk-python3                            2.0.0p5        1d96bebf47a6   7 months ago         2.18GB
     robotmk-cmk-python3                            1.6.0p25       599e8beeb9c7   7 months ago         1.93GB
     robotmk-cmk-python3                            2.0.0p4        71bdfccd584b   7 months ago         2.19GB

### Configure task chooser

(Make sure that the extension "tasks-chooser" is installed.)
Now that you have the Docker image versions, you need to add new entries for each specific CMK version in `.vscode/tasks-chooser.json`:

```
{
    "displayName": "▶︎ Set devcontainer to CMK 2.1.0b4",
    "command": "bash .devcontainer/set_devcontainer_version.sh 2.1.0b4"
},
```

### Choose and Start the VS Code devcontainer

Now it's time to run the container: 

- Run Cmd-Shift-P and select `Select Task...` and chose e.g. "Set devcontainer to CMKv2".
- Run Cmd-Shift-P and select `Run Task...` to run the task. This reconfigures some important files:

```
> Executing task: bash .devcontainer/set_devcontainer_version.sh 2.1.0b4 <

+ Generating devcontainer file for CMK 2.1.0b4...
+ Configuring Python for CMK 2... 
+ Setting debug configuration for CMK 2 

>>> Preparation for Checkmk version 2.1.0b4 finished.
You can now start the devcontainer in VS Code with 'Remote-Containers: Rebuild Container'.
```

- Run Cmd-Shift-P and select `Remote-Containers: Rebuild Container` to start the devcontainer. 

In the VS Code terminal you see the CMK site starting. 
This takes some minutes (at least on my aged Mac). 
During this step, all relevant files for Robotmk get [lsynced](https://axkibe.github.io/lsyncd/) (V1)/symlinked (V1) into the version specific folder of the CMK Docker container. 

**Don't try to install the Robotmk MKP into this container! All files are already there!** 

If lsyncd is not running, do this by hand: `lsyncd .lsyncd`

The devcontainer is ready now.

### Select Python Interpreter 

After the devcontainer has started, you probably have to set the python interpreter in VS Code explicitly. 
This is sometimes a little bit unreliable and must be done manually: 

- ensure that `settings.json` do not contain a `python.pythonPath` setting anymore
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

### Troubleshooting 

#### Devcontainer does not start 

ERROR: The devcontainer does not start; the VS Code `remoteContainers-YYYY-MM-DD` shows: 

     [2022-04-01T13:24:30.960Z] [+] Building 2.5s (3/3) FINISHED                                                
     => [internal] load build definition from Dockerfile                       0.1s
     => => transferring dockerfile: 37B                                        0.0s
     => [internal] load .dockerignore                                          0.0s
     => => transferring context: 2B                                            0.0s
     => ERROR [internal] load metadata for docker.io/library/robotmk-cmk-pyth  2.2s

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
