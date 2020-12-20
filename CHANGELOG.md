# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## Unreleased
### Added
### Changed
### Fixed

## [v0.1.5 - 2020-12-15]
### Added

- Log/Report/XML files of RF tests are now saved with a timestamp to make debugging easier.
  New WATO option for the check plugin to set the number of days log files should be kept. (solves #56)
- Besides UTF-8, there are two more options in WATO to encode the data between agent and server: 
  BASE-64 and zlib compression. The latter is implemented with regard to the upcoming integration 
  of RF HTML logs into Checkmk. As soon they are containing screenshot data, compression is needed.
  Solves #65.

### Changed

- WATO option "includedate": in the past, it contained the end timestamp of the node
  itself. This could lead to misunderstandings if discovery level > 0 was used, because
  last execution for services generated from tests were different. In fact this was true, 
  but the interesting fact is when the _suite_ was executed last, not a subsuite or test. 
  For this reason, the date is now the timestamp of the whole suite execution end.  


### Fixed

- Support UTF-8 (closes #55)
- RF States of Keywords are now preserved and not propagated upwards. This is neccessary because
  keywords like 'Run Keyword And Return Status' can wrap kw execution; a failed kw will not fail 
  the whole test. With this commit, the cmk-side evaluation of the RF restult tree respects the RF
  state of each node, but does not propagate it. (solves #57 and #58)
- Bakery crashes when RobotMK rule does not contain specific suites (closes #45)

## [v0.1.4] - 2020-11-15
### Added

- Bakery/Check: Added spooldir mode; Robotmk plugin can be triggered externally, writes to spooldir of mk agent (#49)
- Check now makes use of HTML badges for WARN/CRIT (#52)

### Changed

- Plugin: Set tmpdir on Windows to a fixed path (#47)
- Improved Logging (#48)

### Fixed

- No graphs when using a discovery level (#53)
## [v0.1.3] - 2020-0-16
### Added 

- Check: Introduce CRITICAL state for thresholds (#22)
- Check: Add WATO option: include execution timestamp into first line (#41)

### Fixed

- Check: Solved check crash when only perfdata rule active, not threshold (#40)

## [v0.1.2] - 2020-07-10

### Fixed

- Bakery: Wrong formatting of variable argument (#38)
- Plugin: Backslash escaping of CheckMK programdata path (#37)

## [v0.1.1] - 2020-07-01

- First Release; Bakery, Plugin and Check are working together


## [vx.x.x] - yyyy-mm-dd
### Added

### Changed

### Fixed

### Removed

### Deprecated
