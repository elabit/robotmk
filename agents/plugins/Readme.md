# checkmk agent plugin for robotmk

## YAML Configuration File
The YAML configuration for robotmk plugin has a gobal section with options globally used for all suites. Possibly this options may overwritten per suite.
Currently the following options are global:
|option| function| default value|
|-------------------|----------------------|---------------|
|outputdir| The directory where the XML outputfile is stored|TBD|
|robotdir| The directiry where the robot suites are living|TBD|
|log| Logfiles|none|
|console| Should be always **none**|none|
|report| Reports generation|none|
Then the yaml configuration has a dictonary named `suites` which contains a dictonary for each suite to be run. The name of key of the dictonaries below `suites` MUST have the same name as the directory below the robot root directory (option `robotdir`). Each suite contains a dictionary with robot options. The keys have the same name as the real options without the `--`. See `robot --help` for explanation.

## Agent Plugin