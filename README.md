# Robotmk
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-4-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

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
   * [Development setup](#development-setup)
      * [Environment setup](#environment-setup)
         * [Starting the VS Code devcontainer](#starting-the-vs-code-devcontainer)
         * [VS Code Build Task](#vs-code-build-task)
      * [Debugging the Robotmk check](#debugging-the-robotmk-check)
      * [Release](#release)
   * [Next developments](#next-developments)
   * [Contributing](#contributing)
   * [License](#license)
   * [Credits/Thanks](#creditsthanks)
      * [Supporters](#supporters)
   * [Contributors <g-emoji class="g-emoji" alias="sparkles" fallback-src="https://github.githubassets.com/images/icons/emoji/unicode/2728.png">✨</g-emoji>](#contributors-)

<!-- Added by: runner, at: Sun Jul 25 08:41:08 UTC 2021 -->

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

### `robotmk.yml` explained

This section is important if you decide not to use the CMK Enterprise edition (CEE) and to generate the `robotmk.yml` file instead by hand or automated by Ansible, Salt etc. 

Example of `robotmk.yml`:

```
global:
  agent_output_encoding: zlib_codec
  cache_time: 960
  execution_interval: 900
  execution_mode: agent_serial
  log_rotation: '14'
  logging: true
  robotdir: /usr/lib/check_mk_agent/robot
  transmit_html: true
suites: {}
```

Explanation: 

* `global:`
  * `agent_output_encoding`: `zlib_codec|utf_8|base64_codec` Choose `zlib_codec`; the other options are for debugging purposes. Zlib guarantees bandwidth saving. 
  * `cache_time`
  * `execution_interval` (only in serial mode) 
  * `execution_mode`: `agent_serial|external`
  * `log_rotation`: Number of days to rotate the logs
  * `logging`: `true|false`
  * `robotdir`: The folder containing `.robot` files and/or suite dirs 
  * `transmit_html`: `true|false` - whether to transmit the Robot HTML log to the CMK server. 
* `suites:` - if Robotmk should execute everythin it can find in `robotdir`, set this to `{}` (empty dict)
  * `suiteid`: either equal to `path` (see below) or, if the same suite must be executed multiple times, a combination of `path` and `tag`
    * `path`: relative path to the `.robot` file or a folder within the `robotdir` (the latter one is highly recommended)
    * `piggybackhost` (optional): if set, the result of this suite will be assigned to another CMK host.
    * `robot_params` (optional): a list of Robot framework command line switches 




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

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="http://kleinski.de"><img src="https://avatars.githubusercontent.com/u/3239736?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Marcus Klein</b></sub></a><br /><a href="https://github.com/simonmeggle/robotmk/issues?q=author%3Akleinski" title="Bug reports">🐛</a></td>
    <td align="center"><a href="https://burntfen.com"><img src="https://avatars.githubusercontent.com/u/910753?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Richard Littauer</b></sub></a><br /><a href="#mentoring-RichardLitt" title="Mentoring">🧑‍🏫</a></td>
    <td align="center"><a href="https://github.com/a-lohmann"><img src="https://avatars.githubusercontent.com/u/9255272?v=4?s=100" width="100px;" alt=""/><br /><sub><b>A. Lohmann</b></sub></a><br /><a href="https://github.com/simonmeggle/robotmk/issues?q=author%3Aa-lohmann" title="Bug reports">🐛</a></td>
    <td align="center"><a href="https://github.com/NimVek"><img src="https://avatars.githubusercontent.com/u/6333118?v=4?s=100" width="100px;" alt=""/><br /><sub><b>NimVek</b></sub></a><br /><a href="https://github.com/simonmeggle/robotmk/issues?q=author%3ANimVek" title="Bug reports">🐛</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!