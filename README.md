# Development documentation

Install checkMK pytest module: 

```
pip install -e /python-pytest-check_mk/
```

Waiting for pull request:

* https://github.com/tom-mi/python-pytest-check_mk/pull/1
* https://github.com/tom-mi/python-pytest-check_mk/pull/2


## install check

Install the check by creating a symlink: 

    ln -s /workspace/robotmk/checks/robotmk /omd/sites/cmk/local/share/check_mk/checks/robotmk

Verify that checkMK can use the robotmk check: 

```
OMD[cmk]:~$ cmk -L | grep robot                                          
robotmk     tcp    (no man page present)
```

## install wato configuration settings

Install the WATO configuration settings by creating a symlink: 

    ln -s /workspace/robotmk/check_parameters_robotmk.py /omd/sites/cmk/local/share/check_mk/web/plugins/wato/check_parameters_robotmk.py


## test

FIXME see 2. "generate test data"

``` 
OMD[cmk]:~$ cmk -IIv robothost1
Discovering services on: robothost1
robothost1:
+ FETCHING DATA
 [agent] Execute data source
 [piggyback] Execute data source
No piggyback files for 'robothost1'. Skip processing.
No piggyback files for '127.0.0.1'. Skip processing.
+ EXECUTING DISCOVERY PLUGINS (44)
systemd_units does not support discovery. Skipping it.
ps_lnx does not support discovery. Skipping it.
ps.perf does not support discovery. Skipping it.
  1 chrony
  1 cpu.loads
  1 cpu.threads
  4 df
  1 diskstat
  3 kernel
  1 kernel.util
  1 livestatus_status
  1 lnx_if
  2 lnx_thermal
  1 mem.linux
  1 mkeventd_status
  1 mknotifyd
  4 mounts
  1 omd_apache
  1 omd_status
  1 postfix_mailq
  1 postfix_mailq_status
  1 robotmk                      <<<<<<<<<<<<<<<
  1 systemd_units.services_summary
  1 tcp_conn_stats
  1 uptime
SUCCESS - Found 31 services, 1 host labels
```