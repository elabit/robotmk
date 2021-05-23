# Robotmk

*A complete solution to integrate **Robot Framework** End2End tests into **Checkmk***

<!-- [![Build Status](https://travis-ci.com/simonmeggle/robotmk.svg?branch=develop)](https://travis-ci.com/simonmeggle/robotmk) ![.github/workflows/github-markdown-toc.yml](https://github.com/simonmeggle/robotmk/workflows/.github/workflows/github-markdown-toc.yml/badge.svg) -->

[![MD TOC](https://github.com/simonmeggle/robotmk/actions/workflows/markdown-toc.yml/badge.svg)](https://github.com/simonmeggle/robotmk/actions/workflows/markdown-toc.yml) [![MKP-Release](https://github.com/simonmeggle/robotmk/actions/workflows/mkp-release.yml/badge.svg)](https://github.com/simonmeggle/robotmk/actions/workflows/mkp-release.yml)
![desc](img/robotmk_banner.png)

<!--ts-->
* [Robotmk](#robotmk)
   * [Description](#description)
   * [State of development](#state-of-development)
   * [Key features/components](#key-featurescomponents)
   * [Usage scenarios](#usage-scenarios)
   * [Requirements](#requirements)
   * [Installation](#installation)
   * [Documentation](#documentation)
   * [Usage](#usage)
      * [Configure what to execute](#configure-what-to-execute)
      * [Integrate the new Robot E2E check into Checkmk](#integrate-the-new-robot-e2e-check-into-checkmk)
      * [Configure the E2E check](#configure-the-e2e-check)
      * [Discovery level: split up a Robot tests into many CMK services](#discovery-level-split-up-a-robot-tests-into-many-cmk-services)
   * [Development setup](#development-setup)
      * [Installation](#installation-1)
      * [Python versions](#python-versions)
      * [tox](#tox)
      * [running tests with tox](#running-tests-with-tox)
      * [Submodule init](#submodule-init)
      * [Debugging the Robotmk check](#debugging-the-robotmk-check)
   * [Next developments](#next-developments)
   * [Contributing](#contributing)
   * [License](#license)
   * [Credits/Thanks](#creditsthanks)
      * [Contributions](#contributions)
      * [Supporters](#supporters)

<!-- Added by: runner, at: Fri May 21 11:19:29 UTC 2021 -->

<!--te-->

## Description

**What is Robotmk?** 

`"Robot Framework + Checkmk = Robotmk"`

* [Robot Framework](https://robotframework.org/) is a generic testing framework. It can test any kind of application with the help of *libraries*. 
* [Checkmk](https://checkmk.com) is a state-of-the-art IT infrastructure monitoring system. 
* **Robotmk** integrates the results of Robot Framework into Checkmk. It bridges the gap between infrastructure and application testing. 

**Why do I need Robotmk?** 

A monitoring system like Checkmk does a very good job to monitor your business' IT infrastructure with checks for Servers, Network devices, etc. 

But in the end - the reason why you are running IT is to *provide a service* to users. 

Therefore you shouldn't only monitor infrastructure, but also *the services*. And most important: do it like they do. Use a real browser, mouse and keyboard strokes. Test from End (the user) to End (your IT infrastructure as a whole). This is called **"End2End"-Testing**.

Robot Framework can automate End2End-Tests for you (and much more). Integrating those tests into Checkmk is a great supplement.

**Robotmk** acts as a bridge between Robot Framework and Checkmk. 

## State of development

**Is Robotmk stable? Can it be used in production?**

Fortunately, the development of Robotmk is driven by customers who believe in the project and use it already in their daily business. This is where worthful feedback and feature requests come from. 

As bugs are getting solved and new features are coming in, there is no guarantee that after installing a new version of Robotmk settings, output formats etc. will be the same or at least compatible with the previous version. We try to communicate this in the [CHANGELOG](./CHANGELOG.md) as detailled as possible. 

Incompatibilities will always be reflected in a major version change. As soon as the major version number is not changing, chances are good that all existing CMK rules for Robotmk will work.  

## Key features/components

* Robotmk **bakery rule** - configures E2E clients:
  * Use the Checkmk WATO rule editor to decide which remote hosts should be deployed with the Robotmk plugin.
  * Define which suites should be executed there and the individual parameters. 
* Robotmk **plugin** - executes RF tests: 
  * integrated into the Checkmk monitoring agent, it is a kind of wrapper for RF tests on the client side. It gets controlled by the robotmk YML file which is created by the bakery. 
* Robotmk **check** - evaluates RF results:
  * evaluates the RF result coming from the Checkmk agent. 
  * 100% configurable by web (WATO), 100% Robot compatible: Robotmk does not require any adaptation to existing Robot tests; they can be integrated in Checkmk without any intervention.
  * powerful pattern-based definition system for "most general" and/or fine granular control of
    * runtime thresholds: get alarms for suites/tests/keywords running too long. 
    * performance data: get graphs for any runtime. Even insidious performance changes can thus be detected.
    * service discovery level: rule-based splitting of Robot Framework results into different Checkmk services ("checks" in Checkmk) - without splitting the robot test. 
    * reduction of the output to the essential needs for an optimum result.

Read the [feature page](https://robotmk.org) of Robotmk to learn about its history, features and advantages. 

## Usage scenarios

**Robotmk** is great for: 

* having both monitoring business-critical applications and infrastructure check within the same monitoring tool (Checkmk)
* monitoring modern apps: Angular, React, Android/iOS based, ... Robot Framework has a long list of well-curated libraries
* monitoring old legacy apps: even the oldest applications can be monitored with Robot Framework by using a image recognition based library. 
* monitoring 3rd party services: there are bunch of libraries to write tests based on REST, SOAP, TCP sockets, SSH, FTP, ...  

## Requirements

Robotmk works with any Checkmk 1.6x and 2.x version and edition (CEE and CRE).

*  Enterprise edition (CEE) is recommended if you want to benefit from the agent bakery system which creates agent installation packages and the Robotmk YAML configuration files. 
* Raw Edition (CRE) also works if you are fine to write this files by hand/generate by some other tool (Ansible etc.). (Nevertheless, consider a worthwile [switch to CEE](https://www.iteratio.com/))

## Installation

You can choose between two ways of installing Robotmk: 

* Installing as [MKP](https://checkmk.com/cms_mkps.html) is the preferred way. 
  * The most recent release can be downloaded here on the [Releases](https://github.com/simonmeggle/robotmk/releases) page
  * The latest MKP *reviewed by tribe29* (the Checkmk guys) can be fetched from [CMK Exchange](https://exchange.checkmk.com/) (not always up to date)
* Installation by hand is only recommended for advanced users who love to get dirty hands. 

![mkp-installation](img/mkpinstall.gif)


Now verify that checkMK can use the robotmk check: 

```
$ su - cmk
OMD[cmk]:~$ cmk -L | grep robot                                          
robotmk     tcp    (no man page present)
```

## Documentation

All Robotmk rules come with a very **detailled and comprehensive context help**. This covers 95% of all information which is needed to work with Robotmk. 

The context help can be shown by clicking on the **book icon** in the top right corner of every Robotmk rule:  

![How to show the context help](img/show_context_help.gif)

## Development setup

### Environment setup 

#### VS Code Build Task

`Ctrl+Shift+B` is bound to `build.sh` which builds the CMK version specific MKP file. 

The resulting MKP can be copied to the host system as follows: 

```
CONTAINER=a596f322c2e8
cd ~/Downloads
docker exec $CONTAINER bash -c "mkdir -p /cmk-mkp; cp /workspaces/robotmk/*.mkp /cmk-mkp"
docker cp $CONTAINER:/cmk-mkp .
```


### Debugging the Robotmk check

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

### Release

`release.sh` is a helper tool which eases (un)releasing a lot. Sometimes a alpha/beta release should to be withdrawn. With the helpf of this script and the github CLI tool (authentication required),

* the release gets deleted
* tags are removed
* develop branch gets checked out
* `chag` undoes the last change to the `CHANGELOG`


```
# unrelease
bash release.sh unrelease 1.1.0-beta
# release
bash release.sh release 1.1.0-beta
```

## Next developments

See the [Github Issues](https://github.com/simonmeggle/robotmk/issues) page for a complete list of feature requests, known bugs etc.

Next development steps will be: 

* It is helpful to have also Robot Logs at hand when there is an alarm. It is planned
  that the Robotmk plugin also collects the Robot HTML logs and transfers them to the
  CMK server. HTML logs could also include (embedded?) screenshots, animated GIF screen-recordings etc. 
  The goal is to implement a service action button which guides you directly to the most
  recent Robot Log. Read more about this idea in [issue #1](https://github.com/simonmeggle/robotmk/issues/1).   
* Create a Docker container to execute Robot tests also in Containers. Expand the agent plugin to trigger Robot containers with API calls to Kubernetes and Docker Swarm to distribute E2E tests.  

## Contributing

If you want to help Robotmk to get better, you're warmly welcomed!

* Fork this project
* Create a feature branch with a name containing the issue number (or submit a new issue first), from the current `develop` branch. 
* Always and often rebase your feature branch from `develop` 
* Pull requests are welcome if they can be merged and solve a problem

## License

**Robotmk** is published unter the [GNU General Public License v3.0](https://spdx.org/licenses/GPL-3.0-or-later.html)

## Credits/Thanks

### Supporters

Thanks to the companies which support the development of Robotmk: 

* [Abraxas Informatik AG](https://www.abraxas.ch/), St. Gallen (CH) -  Jens Dunkelberg
* [ITERATIO GmbH](http://iteratio.com/), Cologne (GER) - Hardy Düttmann
* [comNET GmbH](https://www.comnetgmbh.com), Hannover (GER) - Thorben Söhl
