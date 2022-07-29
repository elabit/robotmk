# Robotmk
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-4-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

*A complete solution to integrate **Robot Framework** End2End tests into **Checkmk***

<!-- [![Build Status](https://travis-ci.com/simonmeggle/robotmk.svg?branch=develop)](https://travis-ci.com/simonmeggle/robotmk) ![.github/workflows/github-markdown-toc.yml](https://github.com/simonmeggle/robotmk/workflows/.github/workflows/github-markdown-toc.yml/badge.svg) -->

[![MD TOC](https://github.com/simonmeggle/robotmk/actions/workflows/markdown-toc.yml/badge.svg)](https://github.com/simonmeggle/robotmk/actions/workflows/markdown-toc.yml) [![MKP-Release](https://github.com/simonmeggle/robotmk/actions/workflows/mkp-release.yml/badge.svg)](https://github.com/simonmeggle/robotmk/actions/workflows/mkp-release.yml)
![desc](img/robotmk_banner.png)

<!--ts-->
- [Robotmk](#robotmk)
  - [Description](#description)
  - [Key features/components](#key-featurescomponents)
  - [Usage scenarios](#usage-scenarios)
  - [State of development](#state-of-development)
  - [Requirements](#requirements)
  - [Installation](#installation)
    - [Installing the Robotmk server component](#installing-the-robotmk-server-component)
    - [Preparing the Robotmk agent](#preparing-the-robotmk-agent)
  - [Documentation](#documentation)
  - [FAQ](#faq)
  - [Learning Resources](#learning-resources)
  - [Talks](#talks)
  - [Professional Support](#professional-support)
  - [Next developments](#next-developments)
  - [Contributing](#contributing)
  - [License](#license)
  - [Credits/Thanks](#creditsthanks)
    - [Supporters](#supporters)
  - [Contributors ✨](#contributors-)

<!-- Created by https://github.com/ekalinin/github-markdown-toc -->
<!-- Added by: runner, at: Wed Jun 29 16:48:04 UTC 2022 -->

<!--te-->

## Description

**What is Robotmk?** 

`"Robot Framework + Checkmk = Robotmk"`

* [Robot Framework](https://robotframework.org/) is a generic testing framework. It can test any kind of application with the help of *libraries*. 
* [Checkmk](https://checkmk.com) is a state-of-the-art IT infrastructure monitoring system. 
* **Robotmk** integrates the results of Robot Framework into Checkmk. It bridges the gap between infrastructure and application testing. 

**The purpose of Robotmk** 

Business applications have to be **available, performant** and **functioning**. 

A monitoring system like Checkmk does a very good job to monitor your business' IT infrastructure with checks for Servers, Network devices, etc. 

But in the end - the reason why you are running IT is to *provide a service* to users. 

Therefore you shouldn't only monitor infrastructure, but also *the services*. And most important: do it like *they* do: Use a real browser, mouse and keyboard strokes. Test from End (the user) to End (your IT infrastructure as a whole). This is called **"End2End"-Testing**.

**Robot Framework** can automate End2End-Tests for you (and much more). 
Integrating those tests into **Checkmk** is a great supplement.
**Robotmk** acts as a bridge between **Robot Framework** and **Checkmk**. 

## Key features/components

The Robotmk project consists of different components working together: 

* Robotmk **WATO pages** for 100% configuration via the checkmk web UI
* Robotmk **bakery rule** - configures E2E clients:
  * Use the Checkmk WATO rule editor to decide which remote hosts should be deployed with the Robotmk plugin.
  * Define which suites should be executed on the test host
  * Parametrize the Robot tests with commandline options of Robot Framework - no CLI needed. 
  * Let Robot Framework repeat failed tests for certain number of attempts. This is a very useful and outstanding feature to get more stable results if the SUT behaves unconsistently. 
* Robotmk **plugins** - executes RF tests: 
  * the `robotmk-runner` is a kind of wrapper for RF tests on the client side. It gets controlled by the robotmk YML file which is created by the bakery. 
  * the `robotmk` plugin is executed as a normal plugin and reads the results written by the runner plugin.
* Robotmk **check** - evaluates RF results:
  * evaluates the RF result coming from the Checkmk agent. 
  * 100% configurable by web (WATO), 100% Robot compatible: Robotmk does not require any adaptation to existing Robot tests; they can be integrated in Checkmk without any intervention.
  * powerful pattern-based definition system for "most general" and/or fine granular control of
    * runtime thresholds: get alarms for suites/tests/keywords running too long. 
    * performance data: get graphs for any runtime. Even insidious performance changes can thus be detected.
    * service discovery level: rule-based splitting of Robot Framework results into different Checkmk services ("checks" in Checkmk) - without splitting the robot test. 
    * reduction of the output to the essential needs for an optimum result.
    * (new) log integration: The check extracts the Robot HTML logs and integrates 
      them into the Checkmk web UI: 

![desc](img/robotmk_html_logs_animated.gif)

*The integration of Robot HTML logs into Checkmk makes it easy to understand why an E2E test failed*

* [Robotmk Keyword Library](https://pypi.org/project/robotframework-robotmk/):
  * Set custom monitoring states
  * Add monitoring metrics
  * Add custom performance data

Read the [feature page](https://robotmk.org) of Robotmk to learn about its history, features and advantages. 

## Usage scenarios

**Robotmk** is great for: 

* having both monitoring business-critical applications and infrastructure check within the same monitoring tool (Checkmk)
* monitoring modern apps: Angular, React, Android/iOS based, ... Robot Framework has a long list of well-curated libraries
* monitoring old legacy apps: even the oldest applications can be monitored with Robot Framework by using a image recognition based library. 
* monitoring 3rd party services: there are bunch of libraries to write tests based on REST, SOAP, TCP sockets, SSH, FTP, ...  
* **Monitoring business relevant services** to save time, cost and revenue. 

## State of development

**Is Robotmk stable? Can it be used in production?**

Robotmk has been used successfully and productively by companies in a wide range of industries since 2020. 

Fortunately, the development of Robotmk is driven by customers who believe in the project and use it already in their daily business. This is where worthful feedback and feature requests come from. 

However, as bugs are getting solved and new features are coming in, there is no guarantee that after installing a new version of Robotmk settings, output formats etc. will be the same or at least compatible with the previous version. We try to communicate this in the [CHANGELOG](./CHANGELOG.md) as detailled as possible. 

Incompatibilities will always be reflected in a major version change. As soon as the major version number is not changing, chances are good that all existing CMK rules for Robotmk will work.  



## Requirements

Robotmk works with any Checkmk 1.6x and 2.x version and edition (CEE and CRE).

* Enterprise edition (CEE) is recommended if you want to benefit from the agent bakery system which creates agent installation packages and the Robotmk YAML configuration files. 
* Raw Edition (CRE) also works if you are fine to write this files by hand/generate by some other tool (Ansible etc.). (Nevertheless, consider a worthwile [switch to CEE](https://www.iteratio.com/))

## Installation

### Installing the Robotmk server component

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

In a distributed monitoring environment, make sure to check the option `Replicate extensions` in the remote site's connection options. 

### Preparing the Robotmk agent 

By using Checkmk Enterprise (highly recommended), the bakery automatically deploys the Robotmk Python plugin scripts as well as `robotmk.yml` automatically.
The only thing to install on the agent beforehand are the following **Python modules** (beside of them the Robot Framework tests need, of course): 

    pip3 install robotframework pyyaml mergedeep python-dateutil

If you are in doubt if CEE is the right choice for you but want to start with the Raw Edition (CRE): the best way to learn what and how to deploy files by hand is to use the ["free" edition of CEE](https://checkmk.com/download?edition=cfe&version=stable) (free up to 25 hosts). You will see then how `robotmk.yml` gets built and where it installs the plugins. 

## Documentation

All Robotmk rules come with a very **detailled and comprehensive context help**. This covers 95% of all information which is needed to work with Robotmk. 

The context help can be shown by clicking on the **book icon** in the top right corner of every Robotmk rule:  

![How to show the context help](img/show_context_help.gif)

## FAQ

The [Frequently asked questions](./FAQ.md) cover a whole range of questions that have arisen time and again in the context of Robotmk.  
They give an insight into why Robotmk differs (or: has to differ) from a regular agent plugin in certain respects.

## Learning Resources

You can find comprehensive learning material to teach you the best practices in End2End monitoring with Checkmk and Robotmk on [the Robotmk Academy](https://www.robotmk.org).

Registration is free and includes three basic course which show the fundamentals of Robotmk. 

Further learning course material is in preparation for 

* Containerized E2E testing with Docker Swarm/Kubernetes/OpenShift
* Best practices ofr using libraries like SeleniumLibrary, Playwright, CryptoLibrary etc.
* fully automated Python environment rollouts with RCC
* Using Robocorp with Robotmk 
* and much more. 

## Talks

* 2020 
  * ROBOCON 2020: [Robot Framework meets Checkmk](https://www.youtube.com/watch?v=7Ks8XSM2Uoc) - *how it all began*
* 2021
  * ROBOCON 2021: [ROBOTMK: TESTING MEETS MONITORING](https://www.youtube.com/watch?v=Asjh7E1xoFI) 
  * FOSDEM 2021: [Extend Checkmk with Robot Framework E2E tests (Why infrastructure monitoring is not enough)](https://archive.fosdem.org/2021/schedule/event/nemorobotmk/)
  * Checkmk Conference #7 2021: [Tech Session - E2E Monitoring](https://www.youtube.com/watch?v=i4W81m-A75U) - *Robotmk's architecture, concept and use cases. Includes an interview with Jens Dunkelberg, Monitoring Team Lead at ABRAXAS Informatik AG (CH), who uses Robotmk for E2E testing of a variety of Swiss government applications.*
  * OSMC 2021 Nuremberg: [Robotmk: You don’t run IT – you deliver services!](https://www.youtube.com/watch?v=3FzFomiQUd8)
* 2022 
  * Checkmk Conference #8 2022: [Robotmk - The future of Synthetic Monitoring with Checkmk](https://www.youtube.com/watch?v=p9G75YCqkOc) - *Robotmk and Checkmk join hands! Presenting the roadmap for the planned integration*

## Professional Support

Need the help of an Expert? Save your time.

No matter at which stage you are with Robot Framework and Robotmk: there are [experienced professionals](https://www.robotmk.org/pages/professionalsupport) who can assist you with: 

* Robotmk best practice for the optimal usage of Robot Framework together with Checkmk * monitoring
* Implementation
* Know-how: all partners are up to date with recent developments and features
* Deep knowledge of configuration tools like Saltstack, Ansible etc. to implement a fully * automated deployment of Robotmk, Robot Framework, Libraries and test code.
* Checkmk best practice

## Next developments

See the [Github Issues](https://github.com/simonmeggle/robotmk/issues) page for a complete list of feature requests, known bugs etc.

As announced at the Checkmk Conference #8 in 2022, Robotmk will become an integral part of Checkmk. The roadmap was presented in detail in this talk ([Robotmk - The future of Synthetic Monitoring with Checkmk](https://www.youtube.com/watch?v=p9G75YCqkOc)) and we are already developing the first features. 

* automatic no-code Robot genration which will extend Checkmk's montoring capabilities 
* low-code suite development (Automation Studio)
* parallel suite execution
* automated test deployment (integration of Robocorp's great [RCC](https://github.com/robocorp/rcc/))
* detailled reporting and result aggregation
* Synthetic monitoring dashboards
* orchestrated Robot execution in:
  * [Robocorp](https://robocorp.com) 
  * Dedicated Kubernetes infrastructure 
  * Docker Swarm


## Contributing

If you want to help Robotmk to get better, you're warmly welcomed!  
See [DEVELOPMENT.md](./DEVELOPMENT.md) for instructions about the environment preconditions. 

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
