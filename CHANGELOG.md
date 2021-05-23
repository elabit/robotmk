# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased


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
