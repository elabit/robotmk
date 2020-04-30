# checkmk agent plugin for robotmk

## YAML Configuration File
The YAML configuration for robotmk plugin has a gobal section with options globally used for all suites. Possibly this options may overwritten per suite.
Currently the following options are global:
|option| function| default value|
|------|------------------------|---------------|
|outputdir| The directory where the XML outputfile is stored|TBD|
|robotdir| The directory where the robot suites are living|TBD|
|log| Logfiles|none|
|console| MUST be always **none** while robotmk runs as a plugin|none|
|report| Reports generation|none|
Then the yaml configuration has a dictonary named `suites` which contains a dictonary for each suite to be run. The name of the key of the dictonaries below `suites` MUST have the same name as the suite directory below the robot root directory (option `robotdir`). Each suite could contain a dictionary with robot options. Option names match robot command line option long names without hyphens so that, for example, `--name` becomes `name` in the yaml configuration. See `robot --help` for explanation. The options are optional and a suite dictonary may could be complete empty.

Most options that can be given from the command line work. An exception is that options --pythonpath, --argumentfile, --help and --version are not supported.

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

A complete robotmk yaml configuration file can look like that:

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