# Execution of runner

## no detached process possible

It has been shown that it is not possible to start a detached process from the controller plugin which survives if the controller exits. 

The assumption is that the checkmk agent always kills the complete process group even if the runner process was started detached: 

```
    def os_popen(self, cmd):
        # FIXME: blocking Agent?
        
        if platform.system() == 'Linux':
            self.loginfo("-> Executing Linux Runner ('%s')" % str(cmd))
            subprocess.Popen(cmd)
        elif platform.system() == 'Windows':
            
            flags = 0
            flags |= 0x00000008  # DETACHED_PROCESS
            flags |= 0x00000200  # CREATE_NEW_PROCESS_GROUP
            flags |= 0x08000000  # CREATE_NO_WINDOW

            pkwargs = {
                'close_fds': True,  # close stdin/stdout/stderr on child
                'creationflags': flags,
            }
            cmd.insert(0, sys.executable)
            self.loginfo("-> Executing Windows Runner ('%s')" % str(cmd))
            P = subprocess.Popen(cmd,**pkwargs)

            pass
```

## alternative ways

An alternative solution must fulfill the following requirements: 
* `agent_serial`: run all suites as configured in the SYSTEM context (=the Agent context)
* `external`: using task scheduler/cron to execute the tests with a special user

### WAY 1: split up controller and runner

A solution is to split the robotmk plugin into two separate files.
Both plugins are located in the custom plugins folder. 

* `robotmk.py` ("Controller")
  * reads state files
  * produces Agent output (stdout) 
* `robotmk-runner.py` ("Runner")
  * in mode `agent_serial`: executes suites as configured in YML files
  * in mode `external`: executes nothing


In both modes, the Controller gets executed minutely because its only job is parsing and printing out data. 

The behaviour of the Runner is depending on the execution mode, defined in `robotmk.yml`:

### agent_serial

The Runner gets executed in the interval set by WATO. This is achieved with an entry in `check_mk.bakery.yml`: 

```
plugins:
 folders: ['$CUSTOM_PLUGINS_PATH$', '$BUILTIN_PLUGINS_PATH$' ]       # ProgramData/checkmk/agent/plugins & Program Files x86/checkmk/service/plugins
    execution:
        - pattern     : '$CUSTOM_PLUGINS_PATH$\robotmk-runner.py'
          cache_age   :  {{ WATO_EXECUTION_INTERVAL }}
          async       : yes
          timeout     : {{ WATO_EXECUTION_INTERVAL-10 }}
```



#### external

The Runner does nothing. It's up to the admin to define the scheduling. 

Ideas to make this as comfortable as possible: 

* Windows:
  * Generate Task definition files (XML) which can be imported into Task Scheduler. In WATO, perhaps also set the windows user the job should run under. 
  * Controller can monitor if there is a scheduled task for each suite ID.
* Linux: 
  * Generate cron jobs
