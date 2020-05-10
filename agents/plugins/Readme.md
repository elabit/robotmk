# checkmk agent plugin for robotmk

The plugin requires a python 3 installation with robotframework installed. If robotframework is not installed the plguin will exit silently except if started in debug mode. The agent plugin will check in the `AgentDirectory` the configuration file `robotmk.cfg`. If the file is not found it will use the default configuration (see YAML configuration file), and start for each file or directory below the 1st level of the directory `robot` which is one level above the `PluginsDirectory` (normally /usr/lib/check_mk_agent/robot in Linux) the robot via the function run from the robot API without any additional options, create in the $tmp folder of the OS a XML output file named the same as the file or directory found and print to stdout each time a section ```<<<robotmk:sep(0)>>>``` followed by the content of the XML file. Finally the XML files in the $tmp folder will be deleted before the plugin exits.

__EXAMPLE:__

Directories:
```
   /usr/lib/check_mk_agent/robot/Suite1
   /usr/lib/check_mk_agent/robot/Suite2
```
Files:
```
   /usr/lib/check_mk_agent/robot/Suite3.robot
```
Will create three files, Suite1.xml, Suite2.xml, Suite3.xml in /tmp and print to stdout three times the section ```<<<robotmk:sep(0)>>>``` followed by the content of each XML file.


If a configuration file is found, it will be read in and each option found in the configuration file will overwrite the default value of the corresponding option. If no suites are defined, the plugin will follow the same approach to start the suites as described above without a configuration file.


If suites are defined in the configuration file, the plugin will start only the defined suites with all configured suite specific options, regardless if there are additional suites in the filesystem. If a piggyback host is defined in teh configuration file, the agent section will contain this host to allow to assign the suite to another as the monitored host.

The plugin could be started with the option --debug to allow command line debugging. It is necessary to export the environment variables `AgentDirectory` and `PluginsDirectory` to simulate the agent. The debug option is not intended to debug the robot tests and will only output plugin specific information. To debug robot tests the robot command could be used.

As by nature the plugin will run longer than the agent timout and frequency settings would allow, the plugin has to run as a cached plugin. Create a directory with number of seconds which are at least higher than the number of seconds the robot testes run below the `PluginsDirectory` and move the robotmk agent plugin to that directory. The agent will then delay the start of the plugin that number of seconds and cache the results. If the plugin is not able to return and update the cached results in time the service will go to stale. AFAIK the agent doesnt check if the plugin is already started and start another session after the cache time expired. This means the process robo needs to be monitored that it doesnt run more than once (To be tested).

For the checkmk CEE and CME edition a rule for the agent bakery will be available wich allows the settings of the cache time, the configuration of the global options and the configuration of the suites to be run with all specific options as described below. The YAML configuration file will be then baked along with the robotmk agent plugin in the installation packages and installed on the monitored host in the configured directories.

To deploy the robot suites the Agent Bakery rule "Deploy custom files with agent" could be used. To make that work the files has to be below /usr/lib/check_mk_agent in Linux and below the Agent installation directoyr in Windows. Epsecially for Windows this needs to be tested with the new Agent Version 1.6 and 1.7.

## YAML Configuration File
The YAML configuration for robotmk plugin has a gobal section with options globally used for all suites. Possibly this options may overwritten per suite.
Currently the following options are global:


|option| function| default value|
|------|------------------------|---------------|
|outputdir| The directory where the XML outputfile is stored|OS $tmp|
|robotdir| The directory where the robot suites are living|Directory robot, one level above PluginsDirectory|
|log| Logfiles|none|
|console| MUST be always **none** while robotmk runs as a plugin|none|
|report| Reports generation|none|


Then the yaml configuration has a dictionary named `suites` which contains a dictionary for each suite to be run. The name of the key of the dictonaries below `suites` MUST have the same name as the suite directory or file below the robot root directory (option `robotdir`). Each suite could contain a dictionary with robot options. Option names match robot command line option long names without hyphens so that, for example, `--name` becomes `name` in the yaml configuration. See `robot --help` for explanation. The options are optional and a suite dictionary may could be complete empty.

Most options that can be given from the command line work. An exception is that options --pythonpath, --argumentfile, --help and --version are not supported.

With the option `host` a piggy back host could be configured to allow to assign the suite to a specific host, other than the host the agent runs on.

Options that can be given on the command line multiple times can be passed as lists. For example:
```yaml
critical:
   - tag1
   - tag2
   - tag3
```

is equivalent to --critical tag1 --critical tag2 --critical tag3. If such options are used only once, they can be given also as a single string like `include: tag`.

Or as dictionaries. For example:

```yaml
variable:
   name1: value1
   name2: value2
```

is equivalent to --variable name1:value1 --variable name2:value2 --variable  name3:value3.

A complete robotmk yaml configuration file can look like this:

```yaml
#Global variables
outputdir: /tmp/robot
robotdir: /usr/lib/check_mk_agent/robot
log:
console:
report:
#Here comes the suites
suites:
   Suite1:
      host: mypiggyhost
      variable:
         name1: value1
         name2: value2
      critical: 
         - tag1
         - tag2
         - tag3
   Suite2:
   Suite3:
```
## Agent Plugin