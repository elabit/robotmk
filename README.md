# RobotMK

## What is RobotMK? 

RobotMK allows you to integrate the results of the great [Robot Framework](https://robotframework.org/) into the monitoring system [CheckMK](https://checkmk.com).

RobotMK consists of mainly two components: 

* `checks/robotmk` check: evaluates the XML output of robot
* `check_parameters_robotmk.py`: WATO configuration page 

## Installation

Currently, there is no MK package to install RobotMK. The simplest way to get this done is 

* clone this repository
* checkout the `dev` branch 
* copy the files into your CMK site

```
$ cp /workspace/robotmk/checks/robotmk /omd/sites/SITENAME/local/share/check_mk/checks/robotmk
$ cp /workspace/robotmk/check_parameters_robotmk.py /omd/sites/SITENAME/local/share/check_mk/web/plugins/wato/check_parameters_robotmk.py
```

Now verify that checkMK can use the robotmk check: 

```
$ su - cmk
OMD[cmk]:~$ cmk -L | grep robot                                          
robotmk     tcp    (no man page present)
```

## Development setup

### Submodule init

All tests rely on the Python test module [python-pytest-check_mk](https://github.com/tom-mi/python-pytest-check_mk), for which two pull requests are waiting. 

As long as the pull reqeusts ([1](https://github.com/tom-mi/python-pytest-check_mk/pull/1) and [2](https://github.com/tom-mi/python-pytest-check_mk/pull/2))  are outstanding, the forked version of `python-pytest-check_mk` is included as a git submodule. `tox` (see next section) takes care about the initialisation, so there is no work for you. 

### Python versions
This project is based on two Python versions: 

* **Python 2.7** - robotmk **check** on the CheckMK Server (CheckMK will be running soon on Python3)
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

### running tests

With `tox` installed now, the tests can be started: 

```
# run tests for the plugin (Python 3.6) and the check (Python 2.7)
tox
# run only plugin tests
tox -e plugin 
# run only check tests
tox -e check
```
