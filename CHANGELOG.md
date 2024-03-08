# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## Unreleased

## 1.4.2 - 2023-05-16

Compatibility patch for Checkmk V2.3

### Changed

* Changed location of bakery script (#234)

## 1.4.1 - 2022-10-21

### Fixed

* agent plugin: fixed bug when piggyback was set, runner crashed

## 1.4.0 - 2022-10-13

### Changed 

* The Robotmk mechanism behind the RF parameter `--exitonfailure` has been optimized and clearly documented in the inline help. If this option was set, the remaining tests that were not executed were previously marked as FAIL or CRITICAL. This is the default in Robot Framework (see [RF user guide](http://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#stopping-when-first-test-case-fails)), but in Checkmk it lead to an alarm not only for the root cause (the failed test), but also for the subsequent errors. 
The behavior of Robotmk has been slightly modified in that the tests omitted due to a previous failure are filtered out and not passed to Checkmk for evaluation anymore. Tests that have not been executed become "stale", and false alarms are thus avoided. (Thanks for the valuable feedback.)


## 1.3.1 - 2022-10-13

### Fixed

* `robotmk-runner.py`  crashed when no re-execution strategy was set at all (closes #209)

## 1.3.0 - 2022-10-05

### Added

* Bakery/Check Plugin: Introduced `complete` as new re-execution mode for suites (#206). This is useful for suites that must always run completely because the tests depend on each other. Thus, a suite OK result is returned not when each test was OK in an arbitrary iteration, but rather in a common iteration. 

### Changed

* Bakery: Introduced "incremental" as a name for the previous re-execution mode. ⚠️ Your existing rules are compatible with that new naming scheme - but they are only saved in the new format after they have been opened and saved (even if there are no changes).

## 1.2.10-beta-1 - 2022-08-23

### Added

* Documentation: Added [FAQs](./FAQ.md) with design decisions
* Check: Added the possibility to temporarily skip the execution of suites by placing a `DISABLED` (case sensitive) file in the suite's root folder. (#192) Robotmk will silently ignore this suite as long the DISABLED file is there. This is the perfect alternative to commenting out the suite in `robotmk.yml` manually). You can also optionally write a reason ("*Need a short break.*") into the file; it will get logged in Robotmk's log: 

```
2022-08-02 22:25:28,449  RMKrunner [48887]    INFO: ####################
2022-08-02 22:25:28,575  RMKrunner [48887]    INFO:  => Suites to start: sleep
2022-08-02 22:25:28,583  RMKrunner [48887]    INFO: ~~~~ Suite ID: sleep ~~~~
2022-08-02 22:25:28,591  RMKrunner [48887] WARNING: Suite 'sleep' is skipped because of the 'DISABLED' flagfile in its suite folder. Reason: Need a short break.
2022-08-02 22:25:28,591  RMKrunner [48887] WARNING: (Be aware that the services in Checkmk will become stale soon.)
2022-08-02 22:25:28,595  RMKrunner [48887]    INFO: ... Quitting Runner, bye. ---
```

### Changed

* Bakery rule: Lowered the log retention threshold (#172) 
* Check: Fixed a bug which lead to metric names without specials chars like umlauts => metric names can slightly change, sorry. (#197)

### Fixed 

* Bakery (V2): Added missing `timeout` to async robotmk execution. `timeout` is now  
set to 5 sec less than the execution interval so that the agent can kill the runner  
before the next execution. (#203)
* Check: Creates host dir for HTML logs; updated inline help for HTML log icons (#201)
* Check: Allow only 1 runtime threshold by setting the other one to 0 (#198)

### Removed

### Deprecated


## 1.2.9 - 2022-03-25

### Fixed

* Check: Changed folder for HTML logs from local to var

## 1.2.8 - 2022-01-13

* Check: Fixed #189 - Critical threshold / value not transferred to perfdata

## 1.2.7 - 2021-12-23

* Agent plugin: Fixed a bug in failed test re-execution when the output XML files 
  were not processed in the correct chronological order. (#185)

## 1.2.6 - 2021-12-13

* Check: performance data are only created for a S/T/K if the RF result is PASS (#177).

## 1.2.5 - 2021-12-13

* Check: Fixed #178 when the suite ran, but procudes a fatal error. 

## 1.2.4 - 2021-12-07

* Check: Check crashes when the suite never ran. In this case, no XML is present
  and no data can be parsed. The corrected behaviour now is to skip the discovery
  of suites but to let the "Robotmk" service report that suites are missing results. (#178)

## 1.2.3 - 2021-11-29

### Fixed

* Check: If results are too old, the check now sets the services to stale (= they
  keep their state, but appear faded), whereas the Robotmk service turns into 
  CRITICAL. 


## 1.2.2 - 2021-11-04

### Fixed

* Check: Crashed when Robot Framework merged results contained HTML; refactored HTML stripping (#176)

## 1.2.1.2 - 2021-09-30

### Changed 

* Improved the documentation for logdir permissions in external mode (#174)

## 1.2.1.1 - 2021-09-29

### Changed

* Improved the documentation for HTML log integration WATO discovery rule

## 1.2 - 2021-09-27

Important note: With this release, Robotmk WATO rules make use of the `Transform` 
class which allows to write update-safe WATO rules. 
This Robotmk release contains a huge change on the WATO rules; therefore it was 
impossible to make it compatible to earlier versions with `Transform`. 
Therefore, installing this particular Robotmk version on earlier versions will 
make existing WATO rules unreadable/unusable for a last time. Make sure to save
your rules (screenshot, JSON dump etc.) 

### Fixed

* Check: Check fails if test/kw status is SKIP or NOT RUN (#168)
* Fix: suppress stdout when merging rerun XMLs with rebot, closes #165
* Fix: Do not collect hidden dirs as suites (closes #130)
* Fix: check crashes if all test attempts of rerunfailed were NOK, endtime not available (fixes #166)
* Agent plugin: fixed missing logstate rotation, robotmk logs now every midnight (fixes #155)
* Agent Plugin: respect piggyback option, assign to multiple hosts (#145)
* Fixed bug in agent plugin when no suite dir is present at all
* Cleanup in v1.6 bakery script

### Added 

* WATO rules now use the `Transform` class under the hood (#164)
* New option "argumentfile" allows to specify RF arguments by multiple files (#154)
* WATO allows now to set output/robot/log directory independently (#105)
* Improved agent plugin logging levels (introduced standard log levels)
* All Robotmk services can be searched by service labels: 
  * all Robotmk services: `robotmk:yes`
  * Robotmk monitoring service: `robotmk/type:robotmk`
  * Robot Framework result service: `robotmk/type:result`
* New option: display action links to Robot HTML log files (#1) (*yeah, issue #1 solved!*)
* New WATO option: option to re-execute failed tests of a suite (#150)


### Changed

* Agent plugin: log rotation accepts values from 1 to 365 (0 and "always" removed) #170
* Agent plugin: the execution of Robot Framework was changed from Python API to 
  CLI mode because API does not allow all command line parameters (like argumentsfile).
  This should not have any impact on existing tests. 
* By default, Robotmk writes ALL log and state files into the agent log dir on Windows
  and into `/var/log/` on Linux. (#105)
* Agent plugin: disable report.html creation (closes #163)
* Test message gets converted to HTML when rebot merged it (#150)
* Robot params in robotmk.yml are now within subkey "robot_params"


### Removed

* Options `critical` and `noncritical` were removed from the WATO page because they
  are not supported by RF4.0 anymore. (#154) 
* Removed Robotmk keywords from agent plugin; better install with pip

## 1.2-beta.3 - 2021-09-26

### Fixed

* Check: Check fails if test/kw status is SKIP or NOT RUN (#168)

### Added 

* New option "argumentfile" allows to specify RF arguments by multiple files (#154)
* WATO allows now to set output/robot/log directory independently (#105)
* Improved agent plugin logging levels (introduced standard log levels)
* All Robotmk services can be searched by service labels: 
  * all Robotmk services: `robotmk:yes`
  * Robotmk monitoring service: `robotmk/type:robotmk`
  * Robot Framework result service: `robotmk/type:result`
* New option: display action links to Robot HTML log files (#1) *yeah, issue #1 solved!* 

### Changed

* Agent plugin: log rotation accepts values from 1 to 365 (0 and "always" removed) #170
* Agent plugin: the execution of Robot Framework was changed from Python API to 
  CLI mode because API does not allow all command line parameters (like argumentsfile).
  This should not have any impact on existing tests. 
* By default, Robotmk writes ALL log and state files into the agent log dir on Windows
  and into `/var/log/` on Linux. (#105)

### Removed

* Options `critical` and `noncritical` were removed from the WATO page because they
  are not supported by RF4.0 anymore. (#154) 

## 1.2-beta.2 - 2021-08-27

### Fixed

* Fix: suppress stdout when merging rerun XMLs with rebot, closes #165
* Fix: Do not collect hidden dirs as suites (closes #130)
* Fix: check crashes if all test attempts of rerunfailed were NOK, endtime not available (fixes #166)
* Agent plugin: fixed missing logstate rotation, robotmk logs now every midnight (fixes #155)
* Agent Plugin: respect piggyback option, assign to multiple hosts (#145)

### Changed 

* Agent plugin: disable report.html creation (closes #163)
* Test message gets converted to HTML when rebot merged it (#150)
  

## 1.2-beta - 2021-08-25

### Fixed

* Fixed bug in agent plugin when no suite dir is present at all
* Cleanup in v1.6 bakery script

### Added 

* New WATO option: option to re-execute failed tests of a suite (#150)

### Changed 

* Robot params in robotmk.yml are now within subkey "robot_params"

### Removed

* Removed Robotmk keywords from agent plugin; better install with pip

## 1.1.1 - 2021-07-29

* Fixed CRLF bug, thanks @NimVek (#146)

## 1.1.1-beta - 2021-07-25

### Fixed 

* Fixed two bugs in 1.6 bakery: double nested key in `robotmk.yml` for piggybackhost and robotdir (#141, #143)
* Fixed a bug on Linux when asynchronous execution failed because the runner could not import the Robotmk module. (#115)
* Fixed a bug in agent plugin (1.6 & 2): piggyback option without effect (#142)
* Fixed a bug on Linux: executable bit was not set on agent plugins (#116)
* Fixed a bug in agent plugin: Client setting UTF-8 not handled (#137)
* Fixed a bug in Check (1.6 & 2): Fixed stale suite result handling (#134)

### Changed

* log_rotation gets stored now properly as int in `robotmk.yml`

### Added 

* Added documentation for `robotmk.yml` format for non CEE users 


## 1.1.0 - 2021-06-14

### Changed

* Changed keyword `Add Robotmk Message` to `Add Monitoring Message` to avoid masking of the word "robotmk" by robot framework (fixes #133)

## 1.1.0-beta.4 - 2021-06-01

### Fixed 

* Fixed bug in agent plugin: robotdir default now gets set before accessing it (Thanks @kleinski, Fixes #127)
* Fixed bug in agent bakery: WATO returns a dict with double key (cant be changed; access this double key) (Thanks @kleinski, Fixes #127)
* Fixed bug in agent plugin: Robotdir globbing catched too much files (closes #130)
* Fixed bug in check: set details to None if no messages present (Thanks @kleinski, fixes #131)

## 1.1.0-beta.3 - 2021-05-31

### Fixed

* Fixed bug in v2 bakery (allow_empty crashes only in 2.0.0p5, not p4) (thanks @kleinski, closes #124)


## 1.1.0-beta.2 - 2021-05-23

### Fixed

* Fixed crash in V2 bakery in external mode - Non yielded for runner (closes #123)
* Fixed bug in v1 bakery: no bin files for external mode (closes #122)


## 1.1.0-beta.1 - 2021-05-23

Checkmk V2 Compatibility release

From now on, there are MKP artifacts for both current Checkmk versions: 

* `robotmk.v1.1.0-cmk1.mkp` - Checkmk 1.6x
* `robotmk.v1.1.0-cmk2.mkp` - Checkmk 2.x

### Added

* Completely rewritten bakery script for Checkmk Version 2 (Bakery API)
* Adapted check script for Checkmk Version 2 (Check API)
* Changed from VM/baremetal development to VS Code devcontainer setup 
* Added Github Workflows for Artifacts and Release Assets

### Changed

* The agent plugins now have the default extension `.py`.
* Agent plugin can auto-determine robotdir, if missing in YML config
* WATO pages are V1 and V2 compatible
* Disabled auto-merging of robomk-keywords into agent plugin
* Changed Logo 

### Fixed

### Removed

* Removed pytest - will rebuild the whole test structure from scratch 

### Deprecated


## v1.0.3 - 2021-04-07

### Added

- Support for Robot Framework Robotmk Keyword library (https://github.com/simonmeggle/robotframework-robotmk) (#112); Keywords supported: `Add Robotmk Message`, `Set Checkmk Test State`

### Fixed

- Fixed wrong XML decoding

## v1.0.2 - 2021-04-07

### Fixed

- Fixed version monitoring: swapped variables, closes #118

## v1.0.1 - 2021-04-07

### Fixed

- Fixed version monitoring (#118)

## v1.0.1 - 2021-04-06

### Fixed

- Two suites on same host were not inventorized correctly (#117)

### Removed

- Removed EXEC_MODE from discovery naming variables - useless 


## v1.0.0-beta - 2021-03-25

**WARNING: This first major release is 100% incompatible with former versions.**
Make sure to export all WATO rules because this version is not able to read the
old data structures.  

- Added WATO option to override the Robotmk service name (#98)
- Separate Logfiles
- Runner/Controller
- Suite Tag 
- Log rotation
- Prefix formatting
- Plugin: Added daily log Robotmk file rotation (#88)
- New Robotmk Service: Perfometer Thresholds Graphs
- splitted Robotmk plugin into controller/runner
- The data transmission from client to server is done with a JSON container 
  structure
- HTML log transport to checkmk server


## v0.1.9 - 2021-01-16

### Changed

- WATO option for check: print monitored runtime even if OK (closes #70)

### Fixed

- Bakery set a wrong binary path on Windows (robotmk plugin was placed in agentdata root dir); created 
  custom file packages for linux and windows. Renamed the custom file packages to `robotmk-windows` and 
  `robotmk-linux` (solves #73)
- Bakery: fixed wrong key value for utf-8 (closes #72)
- Plugin: Robot Logfile rotation "never" crashes (TypeError) (closes #77)
- Badges and Unicode symbols for S/T/K do not represent worst state (closes #78)
- Time of last execution does not work on discovery level (#74)

## v0.1.8 - 2021-01-06

### Fixed

- When using a custom service prefix for discovery, the pattern `%SPACE%` can now
  be used at the prefix end to prevent Multisite cropping the string. (Solves #69)

## v0.1.7 - 2021-01-05

### Fixed/Changed

- Service prefix "Robot" is eliminated. By default, there is *no* prefix at all. 
  It can be overriden by a custom one. By installing this version, your existing Robot 
  services will get new descriptions (=without "Robot"), RRD data tied to the old 
  name will be lost. (closes #50)

## v0.1.6 - 2021-01-05

## Added

- Inventory blacklist: when using discovery level, now it is possible to blacklist certain nodes which should not be 
  inventorized as services. (solves #54)

## v0.1.5 - 2021-01-04
### Added

- Log/Report/XML files of RF tests are now saved with a timestamp to make debugging easier.
  New WATO option for the check plugin to set the number of days log files should be kept. (solves #56)
- Besides UTF-8, there are two more options in WATO to encode the data between agent and server: 
  BASE-64 and zlib compression. The latter is implemented with regard to the upcoming integration 
  of RF HTML logs into Checkmk. As soon they are containing screenshot data, compression is needed. Solves #65.
- Added WATO option `show_submessages` to enable/disable the presence of sub-node messages in tests and suites. 

### Changed

- WATO option "includedate": in the past, it contained the end timestamp of the node
  itself. This could lead to misunderstandings if discovery level > 0 was used, because
  last execution for services generated from tests were different. In fact this was true, 
  but the interesting fact is when the _suite_ was executed last, not a subsuite or test. 
  For this reason, the date is now the timestamp of the whole suite execution end.  


### Fixed

- Support UTF-8, solves problem with german umlaut etc. (closes #55)
- RF States of Keywords are now preserved and not propagated upwards. This is neccessary because
  keywords like 'Run Keyword And Return Status' can wrap kw execution; a failed kw will not fail 
  the whole test. With this commit, the cmk-side evaluation of the RF restult tree respects the RF
  state of each node, but does not propagate it. (solves #57 and #58)
- Bakery crashes when Robotmk rule does not contain specific suites (closes #45)
- Multisite crashes ("no closing quotation") when nasty chars from keyword names get into perflabels (closes #64)

## v0.1.4 - 2020-11-15
### Added

- Bakery/Check: Added spooldir mode; Robotmk plugin can be triggered externally, writes to spooldir of mk agent (#49)
- Check now makes use of HTML badges for WARN/CRIT (#52)

### Changed

- Plugin: Set tmpdir on Windows to a fixed path (#47)
- Improved Logging (#48)

### Fixed

- No graphs when using a discovery level (#53)
## v0.1.3 - 2020-0-16
### Added 

- Check: Introduce CRITICAL state for thresholds (#22)
- Check: Add WATO option: include execution timestamp into first line (#41)

### Fixed

- Check: Solved check crash when only perfdata rule active, not threshold (#40)

## v0.1.2 - 2020-07-10

### Fixed

- Bakery: Wrong formatting of variable argument (#38)
- Plugin: Backslash escaping of Checkmk programdata path (#37)

## v0.1.1 - 2020-07-01

- First Release; Bakery, Plugin and Check are working together


## vx.x.x - yyyy-mm-dd
### Added

### Changed

### Fixed

### Removed

### Deprecated
