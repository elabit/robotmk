# Robotmk

*A complete solution to integrate **Robot Framework** End2End tests into **Checkmk***

[![Build Status](https://travis-ci.com/simonmeggle/robotmk.svg?branch=develop)](https://travis-ci.com/simonmeggle/robotmk) ![.github/workflows/github-markdown-toc.yml](https://github.com/simonmeggle/robotmk/workflows/.github/workflows/github-markdown-toc.yml/badge.svg)

![desc](img/robot_robotmk_checkmk.png)

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

<!-- Added by: runner, at: Wed Mar 10 08:09:19 UTC 2021 -->

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

Fortunately, the development of Robotmk is driven by customers who believe in the project and use already it in their daily business. This is where worthful feedback and feature requests come from. 

Even if they already use Robotmk in production there's no point denying the project is still in an early phase (= major version 0.x). 

As bugs are getting solved and new features are coming in, there is no guarantee that after installing a new version of Robotmk settings, output formats etc. will be the same or at least compatible with the previous version. We try to communicate this in the [CHANGELOG](./CHANGELOG.md) as detailled as possible. 


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

Robotmk works with any Checkmk 1.6x version and edition (CEE and CRE).

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

## Usage

*Caveat: the following information can contain minor differences to the current development state*

### Configure what to execute

This recording shows how easy it is to deploy the Robotmk to the host `robothost1`: 

* Go to the WATO section "Monitoring Agents" -> Rules
* Create a new `Robotmk bakery rule`
  * We set the Cache time to 5 minutes so that the plugin gets executed only every 5 minutes.  
  * `sampletest` is the name of a Robot test in the default agent lib folder (`/usr/lib/check_mk_agent/robot`, configurable). To bring Robot test to the client, use the WATO rule `Deploy custom files with agent` - but there is more to come :-)  
  * As you can see, we can set most of the arguments we could also give to Robot on the CLI: rename the suite, pass variables, call variable files with parameters (yeah), etc...
  * Piggyback allows you to assign the Robot result to another host than `robothost1`


![desc](img/bakery.gif)

* The bakery bakes a new RPM package containing the Robotmk plugin and the Robotmk YML file: 

```
# Created by Check_MK Agent Bakery.
# This file is managed via WATO, do not edit manually or you
# lose your changes next time when you update the agent.

cache_time: 300
console: ''
log: ''
report: ''
robotdir: /usr/lib/check_mk_agent/robot
suites:
  sampletest:
    name: iwantanothername
    variablefile:
    - varfile.py:testing
    variables:
    - var1:value1
    - var2:value2
```
### Integrate the new Robot E2E check into Checkmk

As soon as the new agent is installed on the client, it starts to execute the robot test(s). You will notice that the service "Checkmk Discovery" turns to WARNING because it hs found the first result of out Robot test in the agent output. Let's integrate the new check into the monitoring!

![desc](img/disc.gif)

### Configure the E2E check

Now we use the `Robotmk rule for discovered services` to 

* set a threshold on `Subsuite3` on 5 seconds
* draw performance data for every `Subsuite`

![desc](img/ruleconf.gif)

### Discovery level: split up a Robot tests into many CMK services

Lastly, we decide every "Subsuite" in the Robot test to be represented as an own Checkmk service. 
"Subsuite1-3" are one (1) level deeper from the top result level. Hence, we use the `Robot Framework Discovery Level rule` to set the discovery level from 0 (top level) to 1. 


Why would you want to do this? 

* See every test part in a **dedicated monitoring check** with its **own state**
* Perhaps you already have **complex Robot tests** and you do not want to tear them apart 
* Send **notifications for certain parts** of the tests to individual contacts
* Generate **reports about different parts** of the test (e.g. open application, login, report generation)

![desc](img/disc-level.gif)

## Development setup

### Installation 

It is assumed that you are developing on a Linux host which already has Checkmk installed. Instead of copying the files into the site (as described in [Installation](#installation)), just create symlinks (`ln -s `) to the apropriate files and directories. 

### Python versions
This project is based on two Python versions: 

* **Python 2.7** - robotmk **check** on the Checkmk Server (Checkmk will be running soon on Python3)
* **Python 3.6** - robotmk **plugin** on the Robot test host

To run all tests, make sure that you have installed both versions on your machine. 

### tox 

[tox](https://tox.readthedocs.io/en/latest/index.html) manages the virtual envs for us to run tests both for check and plugin within their proper environment. 

First, make sure that you have `tox` installed on your system. It is perfect to install tox in a virtual environment: 

```
~$ virtualenv ~/venv-tox
created virtual environment CPython2.7.5.final.0-64 in 140ms
  creator CPython2Posix(dest=/root/venv-tox, clear=False, global=False)
  seeder FromAppData(download=False, pip=latest, setuptools=latest, wheel=latest, via=copy, app_data_dir=/root/.local/share/virtualenv/seed-app-data/v1.0.1)
  activators PythonActivator,CShellActivator,FishActivator,PowerShellActivator,BashActivator
~$ . ~/venv-tox/bin/activate
(venv-tox) ~$ pip install tox
(venv-tox) ~$ tox --version
3.15.1 imported from /root/venv-tox/lib/python2.7/site-packages/tox/__init__.pyc
```

### running tests with tox

With `tox` installed now, the tests can be started: 

```
# run tests for the plugin (Python 3.6) and the check (Python 2.7)
tox
# run only plugin tests
tox -e plugin 
# run only check tests
tox -e check
```

### Submodule init

All tests rely on the Python test module [python-pytest-check_mk](https://github.com/tom-mi/python-pytest-check_mk), for which two pull requests are waiting. 

As long as the pull reqeusts ([1](https://github.com/tom-mi/python-pytest-check_mk/pull/1) and [2](https://github.com/tom-mi/python-pytest-check_mk/pull/2))  are outstanding, the forked version of `python-pytest-check_mk` is included as a git submodule. `tox` (see next section) takes care about the initialisation, so there is no work for you. 

The manual step to update the submodule is: 
``` 
git submodule update --init --recursive
```

### Debugging the Robotmk check

`ipdb` is a great cmdline debugger for Python. In the following example it is shown how to execute the Robotmk check within the cmk context. 
A breakpoint in line 120 is set with "b":  

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

## Next developments

See the [Github Issues](https://github.com/simonmeggle/robotmk/issues) page for a complete list of feature requests, known bugs etc.

Next development steps will be: 

* Feedback from customers has shown that there is a need for checking the check of Robotmk itself. 
  This means to monitor if the plugin writes spoolfiles in the proper interval. If not, 
  this is currently very hard to detect because only the MK discovery check warns that the 
  `<<<robotmk>>>` agent section is missing. This meta-process will be called `robotmk-master`. 
  Read more about this in [issue 59](https://github.com/simonmeggle/robotmk/issues/59)
* It is helpful to have also Robot Logs at hand when there is an alarm. It is planned
  that the Robotmk plugin also collects the Robot HTML logs and transfers them to the
  CMK server. HTML logs could also include (embedded?) screenshots, animated GIF screen-recordings etc. 
  The goal is to implement a service action button which guides you directly to the most
  recent Robot Log. Read more about this idea in [issue #1](https://github.com/simonmeggle/robotmk/issues/1).   
* Create a complete Docker-based test setup which covers all test scenarios. Why not test Robotmk's functionality with Robot/Selenium itself.
* Create a Docker container to execute Robot tests also in Containers. Expand the agent plugin to trigger Robot containers with API calls to Kubernetes and Docker Swarm to distribute E2E tests.  
* Create dynamic area-stacked performance graphs in the Checkmk grapher. (No, I won't do this for PNP4Nagios...)

## Contributing

If you want to help Robotmk to get better, you're warmly welcomed!

* Fork this project
* Create a feature branch with a name containing the issue number (or submit a new issue first), from the current `develop` branch. 
* Always and often rebase your feature branch from `develop` 
* Pull requests are welcome if they can be merged and solve a problem

## License

**Robotmk** is published unter the [GNU General Public License v3.0](https://spdx.org/licenses/GPL-3.0-or-later.html)

## Credits/Thanks

### Contributions

Thanks to the following people who help to make Robotmk better by submitting code: 

* Michael FRANK (contributed to the agent plugin)
* Guillaume DURVILLE (contributed to the bakery rule)

### Supporters

Thanks to the companies which support the development of Robotmk: 

* [ITERATIO GmbH](http://iteratio.com/), Cologne (GER) - Hardy DÜTTMANN
* [comNET GmbH](https://www.comnetgmbh.com), Hannover (GER) - Thorben Söhl
* [Abraxas Informatik AG](https://www.abraxas.ch/), St. Gallen (CH) -  Jens Dunkelberg
