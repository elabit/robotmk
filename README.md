# Robotmk V2

!!! ALPHA - NOT FOR PRODUCTION !!!

## USAGE

- copy `agent/config/robotmk` into `%Programdata%/checkmk/agent/config/robotmk`
- copy `agent\plugins\robotmk-ctrl.ps1` into `%Programdata%/checkmk/agent/plugins/`

Add to `C:\ProgramData\checkmk\agent\bakery\check_mk.bakery.yml``:


Now start the CMK agent.

`robotmk-ctrl.ps1` (logfile: `C:\ProgramData\checkmk\agent\log\robotmk\robotmk-ctrl.log`) will
- copy itself and the service stub exe to `ProgamData/checkmk/robotmk/RobotmkAgent.exe/.ps1` (is not present)
- Register and start the RobotmkAgent service (if not present/running)
- touch the deadman switch file

`RobotmkAgent.ps1` (logfile: `C:\ProgramData\checkmk\agent\log\robotmk\RobotmkAgent.log`) is started by the service and will
- create the RCC environment for Robotmk (if not present)
- start the Robotmk agent task in RCC
  - the agent runs in a loop and executes Robotmk in sub-processes (not implemented yet)
  - Dummy activity of the Robotmk Agent can be seen in `ProgramData/agent/tmp/robotmk/` (bulk creation of tmp files)
- monitor the deadman switch file (and exit if the file ages out)


---



- ExitHandler Powershell: https://chat.openai.com/chat/738cfffa-39ab-4368-bc20-b1a8e53546d8


ROBOTMK_COMMON_path__prefix
