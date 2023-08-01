[CmdletBinding(DefaultParameterSetName = 'Monitor')]
Param(
  # Ref bbc7b0e
  [Parameter(ParameterSetName = 'Monitor', Mandatory = $false)]
  [Switch]$Monitor = $($PSCmdlet.ParameterSetName -eq 'Monitor'), # Default action
  # Ref 1b0b0b0
  [Parameter(ParameterSetName = 'Status', Mandatory = $false)]
  [Switch]$Status, # Get the current service status
  # # Ref 2be2e2e
  [Parameter(ParameterSetName = 'Start', Mandatory = $true)]
  [Switch]$Start, # Start the service
  # Ref 3c3c3c3
  [Parameter(ParameterSetName = 'Test', Mandatory = $true)]
  [Switch]$Test, # Start the Scheduler in foreground, without service
  # Ref 9466b0
  [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
  [Switch]$Stop, # Stop the service
  # Ref 728a05
  [Parameter(ParameterSetName = 'Restart', Mandatory = $true)]
  [Switch]$Restart, # Restart the service
  # Ref c8466e
  [Parameter(ParameterSetName = 'Install', Mandatory = $false)]
  [Switch]$Install,
  # Ref 863zb3
  [Parameter(ParameterSetName = 'Remove', Mandatory = $true)]
  [Switch]$Remove, # Uninstall the service



  # -- Arguments for SCM
  # Ref 825fb1
  [Parameter(ParameterSetName = 'SCMStart', Mandatory = $true)]
  [Switch]$SCMStart, # Process SCM Start requests (Internal use only)
  # Ref 4d4d4d4
  [Parameter(ParameterSetName = 'Run', Mandatory = $true)]
  [Switch]$Run, # Same as Test, but in background, without service
  # Ref 765fb12
  [Parameter(ParameterSetName = 'SCMStop', Mandatory = $true)]
  [Switch]$SCMStop, # Process SCM Stop requests (Internal use only)
  # Ref bba3224
  [Parameter(ParameterSetName = 'Service', Mandatory = $true)]
  [Switch]$Service, # Run the service (Internal use only)

  [Parameter(ParameterSetName = 'Control', Mandatory = $true)]
  [String]$Control = $null, # Control message to send to the service

  [Parameter(ParameterSetName = 'Version', Mandatory = $true)]
  [Switch]$Version              # Get this script version
)



$currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (!($currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))) {
  Write-Host "ERROR: Script can only run with administrative privileges."
  Exit 1
}


# Determine the given mode
$script_arg = $PSBoundParameters.Keys | Where-Object { $PSBoundParameters[$_] } | Select-Object -First 1
if (-not $script_arg) {
  $script_arg = "Monitor"
}
$argv0 = Get-Item $MyInvocation.MyCommand.Definition
$script = $argv0.basename               # Ex: PSService
$scriptName = $argv0.name               # Ex: PSService.ps1
$scriptFullName = $argv0.fullname       # Ex: C:\Temp\PSService.ps1
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path   # Ex: C:\Temp
$scriptVarfile = "$script.env"

#$scriptname = (Get-Item -Path $MyInvocation.MyCommand.Path).PSChildName
# read mode from args or set to 0
#$MODE = if ($args[0]) { $args[0] } else { $nul };
#$PPID = if ($args[1]) { $args[1] } else { $nul };

$DEBUG = $true
#$DEBUG = $false

# Controls if script should output all log messages on console
if ($Test) {
  $RunningInBackground = $false
}
else {
  $RunningInBackground = $true
}

# ==============================================================================
#   __  __          _____ _   _
#  |  \/  |   /\   |_   _| \ | |
#  | \  / |  /  \    | | |  \| |
#  | |\/| | / /\ \   | | | . ` |
#  | |  | |/ ____ \ _| |_| |\  |
#  |_|  |_/_/    \_\_____|_| \_|
# ==============================================================================

# TODO: After RobotmkScheduler has been copied, produce agent output!

function main() {
  SetScriptVars

  Ensure-Directory $RMKlogdir
  Ensure-Directory $RMKTmpDir
  Ensure-Directory $ROBOCORP_HOME
  Ensure-Directory $RMKSchedulerInstallDir
  # TODO: Test if LogConfiguration can be enabled
  LogConfiguration


  if ($scriptname -match ".*${RMK_ControllerName}$") {
    # CMK Agent/User calls robotmk-ctrl.ps1:
    # - Monitor (default action if no arg)
    #   -> Install (installs/ensures Windows service 'RobotmkScheduler')
    #   -> Start
    #      -> Starts Windows Service 'RobotmkScheduler'
    #         -> RobotmkScheduler.exe (C# stub) - function OnStart()
    #            (-> RobotmkScheduler.ps1 -SCMStart, see below [**])
    # - produce Agent output
    CLIController

  }
  elseif ($scriptname -match ".*${RMKSchedulerName}$") {
    # (e.g. RobotmkScheduler.ps1 -SCMStart/-SCMStop)
    # !Was started from Windows SCM via Robotmk.exe, C# stub (see above [**])
    # - SCMStart
    #   -> Starts RobotmkScheduler.ps1 -Service
    #      -> run the Robotmk Python Scheduler, process workload loop
    # - Service (Start Service Routine, called by -SCMStart)
    # - Run (Start Scheduler Routine directly, called by -Service)
    # - SCMStop
    #   -> Stops RobotmkScheduler.ps1 -Service.
    SCMController
  }
  else {
    LogError "Script name '$scriptname' cannot be evaluated. Exiting."
  }
}

# --------------------------------------------------------------------------
# The 3 main functions

function RMKOutputProducer {
  LogInfo "--- Starting Robotmk Agent Output mode"
  if (RCCIsAvailable) {
    $blueprint = GetCondaBlueprint $conda_yml
    if ( IsRCCEnvReady $blueprint ) {
      LogInfo "Robotmk RCC environment is ready to use, Output can be generated"
      # Ref 9177b1b
      RunRobotmkTask "output"
      #$output_str = [string]::Concat($output)
      foreach ($line in $output) {
        Write-Host $line
      }
    }
    else {
      LogInfo "RCC environment is NOT ready to use. Waiting for Controller to build the environment. Exiting."
      #TODO: Produce some interesting output as long as the RCC env is not ready?
    }
  }
  else {
    # TODO: If no RCC, use native python execution
    Write-Host "TODO: finalize native python execution"
  }
}

function CLIController {
  # TODO: Where call RMKOutputProducer?
  # Workaround for PowerShell v2 bug: $PSCmdlet Not yet defined in Param() block
  $Monitor = ($PSCmdlet.ParameterSetName -eq 'Monitor')

  # All following functions are to control console output depending on the mode

  # Ref bbc7b0e
  if ($Monitor) {
    # -Monitor is the default mode and can be omitted (the Checkmk Agent calls the script w/o args).
    # On each call, renew the deadman file
    TouchFile $controller_deadman_file	"Controller deadman file"

    # This mode installs the service and starts it if not already running.
    RMKSchedulerMonitor
    return
  }

  # Ref 1b0b0b0
  if ($Status) {
    Write-ServiceStatus
    return
  }

  # Ref 2be2e2e
  if ($Start) {
    # Starts the service
    Write-Host "Starting service $RMKSchedulerServiceName"
    RMKSchedulerStart
    LogInfo "Waiting for Processes to start..."
    Start-Sleep  5
    Write-ServiceStatus
    return
  }

  # Ref 3c3c3c3
  if ($Test) {
    # the user starts the ps.1 script with -Test (only for testing!!)
    # (Scheduler runs without service, logs to foreground)
    if (IsProcessRunning "%RobotmkScheduler.exe") {
      Write-Host "$RMKSchedulerServiceName seems to run. You cannot start another instance in foreground. Exiting."
      exit 1
    }
    else {
      # Starts scheduler in foreground and logs to console and file
      RMKSchedulerTester
    }
    return

  }

  # Ref 9466b0
  if ($Stop) {
    Write-Host "Stopping service $RMKSchedulerServiceName"
    RMKSchedulerStop
    Write-ServiceStatus
    return
  }

  # Ref 728a05
  if ($Restart) {
    Write-Host "Restarting service $RMKSchedulerServiceName"
    RMKSchedulerRestart
    # TODO: Replace sleep by some retry function?
    LogInfo "Waiting for Processes to start..."
    Start-Sleep  5
    Write-ServiceStatus
    return
  }

  # Ref c8466e
  if ($Install) {
    Write-Host "Installing service $RMKSchedulerServiceName"
    if (RMKSchedulerServiceScriptNeedsUpdate) {
      RMKSchedulerRemove
      RMKSchedulerInstall
    }

    Write-ServiceStatus
    return
  }

  # Ref 863zb3
  if ($Remove) {
    Write-Host "Stopping service $RMKSchedulerServiceName"
    RMKSchedulerStop
    Write-Host "Removing service $RMKSchedulerServiceName"
    RMKSchedulerRemove
    Write-ServiceStatus
    return
  }



  # Usage TODO
}


function SCMController() {
  # Ref 825fb1
  if ($SCMStart) {
    # Ref 22db44: the SCM calls OnStart() function in the C# stub.
    # This calls the ps1 with arg -Service (see below)
    RMKSchedulerSCMStart
  }
  # Ref 4d4d4d4
  if ($Run) {
    # (Same as -Test, but silent. Scheduler runs without service. This mode
    # is called internally by the Servicescript 'RobotmkScheduler.ps -Service'
    # to begin the actual workload loop.)
    RMKSchedulerRunner
  }
  # Ref 765fb12
  if ($SCMStop) {
    # Ref 6e3aaf: the SCM calls OnStop() function in the C# stub.
    # This calls the ps1 with arg -SCMStop
    RMKSchedulerSCMStop
  }
  # Ref bba3224
  if ($Service) {
    # Called by 'RobotmkScheduler.ps1 -SCMStart' (which was called from the C# stub.)
    # In this mode, the script starts the workload loop, but listens at the same
    # for control messages from the service stub (e.g. Stop)
    RMKSchedulerService
  }

  if ($Control) {
    # Send a control message to the service (only for debugging)
    Send-PipeMessage $RMKSchedulerPipeName $control
    return
  }
}

# --------------------------------------------------------------------------

#    _____ _      _____
#   / ____| |    |_   _|
#  | |    | |      | |
#  | |    | |      | |
#  | |____| |____ _| |_
#   \_____|______|_____|

# Functions which can be executed by the CMK Agent or the user.
# (Called from CLIController)

# Ref bbc7b0e
function RMKSchedulerMonitor {
  $out = @()
  $out += $CMKAgentSection
  #$out += $SubsecController.Replace("xxx", "begin")
  $status = RMKSchedulerStatus
  if ($status -ne "Stopped" -and $status -ne "Running" -and $status -ne "Not Installed") {
    # We have some undefined state.
    LogWarn "Trying to clean up everything."
    RMKSchedulerStop
    $status = RMKSchedulerStatus
    if ($status -ne "Stopped") {
      LogWarn "Service $RMKSchedulerServiceName could not be stopped gracefully. Trying to force it."
      RMKSchedulerStop
      if ((RMKSchedulerStatus) -ne "Stopped") {
        LogError "Fatal: Service $RMKSchedulerServiceName could not be forced to stop."
        LogInfo "Exiting now."
        # TODO: Return status to Checkmk Agent?
        $out += "Fatal: Service $RMKSchedulerServiceName could not be forced to stop."
        return ($out -join "`r`n" | Out-String)
      }
    }

  }
  # Ref 521188
  # AT THIS POINT, the service is either
  # - Running
  # - Stopped  OR
  # - Not Installed
  if ($status -eq "Stopped" -or $status -eq "Not Installed") {
    LogDebug "Service $RMKSchedulerServiceName is $status. "
    if (RMKSchedulerServiceScriptNeedsUpdate) {
      RMKSchedulerRemove
      RMKSchedulerInstall
    }
    RMKSchedulerStart
  }
  elseif ($status -eq "Running") {
    LogDebug "Service $RMKSchedulerServiceName is running."
    # Re-Initializes the service if necessary (Stop/Start/SaveHash)
    if ((RMKSchedulerServiceScriptNeedsUpdate) -or (RCCEnvNeedsUpdate)) {
      ResetRMKSchedulerService
    }
    else {
      # No change ocurred. Nothing to do, we can leave
      $out += "OK: Service $RMKSchedulerServiceName is running and up-to-date."
      return ($out -join "`r`n" | Out-String)
    }
  }

  # Finally, check if the service is _really_ running.
  # Give the processes some time to appear...
  # TODO: Replace sleep by some retry function?
  LogInfo "Waiting for Processes to start..."
  Start-Sleep  5
  $status = RMKSchedulerStatus
  if ($status -ne "Running") {
    LogError "Service $RMKSchedulerServiceName could not be started. "
    LogInfo "Exiting."
    $out += "Service $RMKSchedulerServiceName could not be started."
  }
  else {
    $out += "OK: Service $RMKSchedulerServiceName was just started."
  }
  return ($out -join "`r`n" | Out-String)



  # TODO: Produce CMK Agent output
  # - is RCC environment up-to-date or building right now?
}


function RMKSchedulerServiceScriptNeedsUpdate {
  # Check if the service script is up-to-date
  # If the Controller script is NEWER than the service script, the service must be removed and reinstalled.
  try {
    if ((Get-Item $RMKSchedulerFullName -ea SilentlyContinue).LastWriteTime -lt (Get-Item $scriptFullName -ea SilentlyContinue).LastWriteTime) {
      LogDebug "Service $RMKSchedulerServiceName is already installed, but $scriptName is newer than $RMKSchedulerName. Service requires upgrade"
      return $true
    }
    else {
      LogDebug "Service $RMKSchedulerServiceName is up-to-date"
      return $false
    }
  }
  catch {
    # This is the normal case here. Do not throw or write any error!
    LogDebug "Installation of Service executables into $RMKSchedulerInstallDir is necessary" # Also avoids a ScriptAnalyzer warning
    return $true
  }

}

# Ref c8466e
function RMKSchedulerInstall {
  # Install the service
  # Check if the Service uses an outdated script file
  LogInfo "Installing the following files for service $RMKSchedulerServiceName into $RMKSchedulerInstallDir :"
  if (!(Test-Path $RMKSchedulerInstallDir)) {
    New-Item -ItemType directory -Path $RMKSchedulerInstallDir | Out-Null
  }
  # RobotmkScheduler.ps1: Copy the service script into the installation directory
  if ($ScriptFullName -ne $RMKSchedulerFullName) {
    LogInfo "- Copying myself to $ScriptName"
    Copy-Item $ScriptFullName $RMKSchedulerFullName
  }
  # RobotmkScheduler.ps1.env: store the environment variables because they are not known to the service
  WriteRMKVars $RMKSchedulerInstallDir $RMKScheduler


  # RobotmkScheduler.exe: Generate the binary the C# source embedded in this script
  try {
    LogDebug "- Installing C# Service Stub $RMKSchedulerExeName"
    # Uncomment the following line to debug the C# stub into a text file
    #$source | Out-File "${RMKSchedulerExeFullName}_csharp.txt"
    # The C code in $source contains variables which are replaced and
    # written into the .exe File.
    Add-Type -TypeDefinition $source -Language CSharp -OutputAssembly $RMKSchedulerExeFullName -OutputType ConsoleApplication -ReferencedAssemblies "System.ServiceProcess" -Debug:$false
  }
  catch {
    $msg = $_.Exception.Message
    LogError "Failed to create the $RMKSchedulerExeFullName service stub. $msg"
    exit 1
  }
  # Register the service
  LogInfo "Registering service $RMKSchedulerServiceName (user: LocalSystem)"
  $pss = New-Service $RMKSchedulerServiceName $RMKSchedulerExeFullName -DisplayName $RMKSchedulerServiceDisplayName -Description $RMKSchedulerServiceDescription -StartupType $RMKSchedulerServiceStartupType
  #$pss = New-Service $RMKSchedulerServiceName $RMKSchedulerExeFullName -DisplayName $RMKSchedulerServiceDisplayName -Description $RMKSchedulerServiceDescription -StartupType $RMKSchedulerServiceStartupType -DependsOn $RMKSchedulerServiceDependsOn

}

# Ref 2be2e2e
function RMKSchedulerStart {
  #
  LogInfo "Starting service $RMKSchedulerServiceName"
  Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1002 -EntryType Information -Message "$scriptName -Start: Starting service $RMKSchedulerServiceName"
  # SCM starts the service now. This calls function OnStart() inside of the .exe stub.
  # See the C# code at ref 5f8dda
  # Watch the Windows Application Event Log (Source: RobotmkScheduler) for messages from this service stub.
  # TODO: This writes "Waiting for xxx to start.." on stdout. This is bad because this becomes Agent output!
  Start-Service $RMKSchedulerServiceName
}

# Ref 3c3c3c3
function RMKSchedulerTester {
  # Starts scheduler in foreground and logs to console and file
  RMKScheduler
}

# Ref 4d4d4d4
function RMKSchedulerRunner {
  # STarted by RobotmkScheduler.ps1 -Service
  RMKScheduler
}

# Ref 9466b0
function RMKSchedulerStop {
  # Ref 6e3aaf
  # The user tells us to stop the service.
  try {
    # Stop the service
    LogInfo "Stopping service $RMKSchedulerServiceName"
    Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1004 -EntryType Information -Message "$scriptName -Stop: Stopping service $RMKSchedulerServiceName"
    Stop-Service $RMKSchedulerServiceName -ErrorAction SilentlyContinue
  }
  catch {
    # This is the normal case here. Do not throw or write any error!
    LogDebug "Nothing to stop. Service $RMKSchedulerServiceName is not running" # Also avoids a ScriptAnalyzer warning
  }

  LogDebug "Killing all processes of service $RMKSchedulerServiceName ..."
  KillProcessByCmdline "%$RMKSchedulerName%-SCMStart%"
  KillProcessByCmdline "%$RMKSchedulerName%-Service%"
  KillProcessByCmdline "%$RMKSchedulerName%-Run%"
  KillProcessByCmdline "%robotmk.exe agent fg%"
  KillProcessByCmdline "%robotmk.exe agent scheduler%"
  KillProcessByCmdline "%$RMKSchedulerExeName%"
  # SCM will now call the OnStop() method of the service, which will call the Script with -SCMStop
}

function RMKSchedulerRestart {
  # Restart the service
  RMKSchedulerStop
  if (RMKSchedulerServiceScriptNeedsUpdate) {
    RMKSchedulerRemove
    RMKSchedulerInstall
  }
  RMKSchedulerStart
}

function RMKSchedulerStatus {
  # Get the current service status
  $spid = $null
  $process_pattern = ".*$RMKSchedulerFullNameEscaped.*-Run"
  # Search for RobotmkScheduler.ps1 -Service (this is the process doing the actual service work)
  # See Ref 8b0f1a
  $processes = @(Get-WmiObject Win32_Process -filter "Name = 'powershell.exe'" | Where-Object {
      #$_.CommandLine -match ".*$RMKSchedulerFullNameEscaped.*-Service"
      $_.CommandLine -match $process_pattern
    })
  foreach ($process in $processes) {
    # There should be just one, but be prepared for surprises.
    $spid = $process.ProcessId
    LogDebug "$RMKSchedulerServiceName is running (PID $spid)"
  }
  # if (Test-Path "HKLM:\SYSTEM\CurrentControlSet\services\$RMKSchedulerServiceName") {}
  try {
    $pss = Get-Service $RMKSchedulerServiceName -ea stop # Will error-out if not installed
  }
  catch {
    "Not Installed"
    return
  }

  if (($pss.Status -eq "Running") -and (!$spid)) {
    # This happened during the debugging phase
    LogError "Undefined Service state: $RMKSchedulerServiceName is started in SCM, but no PID found for '$process_pattern'."
    return "noPID"
  }
  else {
    $status = [String]$pss.Status
    #LogInfo "$RMKSchedulerServiceName is $status"
    # return status as string
    return $status
  }
}

# Ref 863zb3
function RMKSchedulerRemove {
  # Uninstall the service
  # Check if it's necessary
  # TODO: check if RobotmkScheduler.exe is runnning; if so, kill it
  LogInfo "Removing service $RMKSchedulerServiceName from SCM..."
  try {
    $pss = Get-Service $RMKSchedulerServiceName -ea stop # Will error-out if not installed
    Stop-Service $RMKSchedulerServiceName # Make sure it's stopped
    # In the absence of a Remove-Service applet, use sc.exe instead.
    $msg = sc.exe delete $RMKSchedulerServiceName
    if ($LastExitCode) {
      LogError "Failed to remove the service ${serviceName}: $msg"
    }
  }
  catch {
    LogDebug "Service ${RMKSchedulerServiceName} is already uninstalled"
    return
  }
  finally {
    # Remove the installed files
    if (Test-Path $RMKSchedulerInstallDir) {
      foreach ($ext in ("exe", "pdb", "ps1", "env")) {
        $file = "$RMKSchedulerInstallDir\$RMKSchedulerServiceName.$ext"
        if (Test-Path $file) {
          LogDebug "- Deleting file $file"
          Remove-Item $file
        }
      }
      LogDebug "- Removing Scheduler service directory $RMKSchedulerInstallDir"
      Remove-Item $RMKSchedulerInstallDir -Force -Recurse
    }
  }
}


#    _____ ______ _______      _______ _____ ______
#   / ____|  ____|  __ \ \    / /_   _/ ____|  ____|
#  | (___ | |__  | |__) \ \  / /  | || |    | |__
#   \___ \|  __| |  _  / \ \/ /   | || |    |  __|
#   ____) | |____| | \ \  \  /   _| || |____| |____
#  |_____/|______|_|  \_\  \/   |_____\_____|______|


# The following functions are NOT meant to be called by the user.
# They are called by the service stub RobotmkScheduler.exe
# Dispatching by SCMController (active when Script is ServiceScript = RobotmkScheduler.ps1)

# Ref 825fb1
function RMKSchedulerSCMStart {
  # Ref 22db44: Param -SCMStart
  # The SCM tells us to START the service
  # Do whatever is necessary to start the service script instance
  LogInfo "$scriptFullName -SCMStart: Starting script '$scriptFullName' -Service"
  Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1001 -EntryType Information -Message "$scriptName -SCMStart: Starting script '$scriptFullName' -Service"
  # Ref 8b0f1a
  # This commandline is searched for in function RMKSchedulerStatus()
  Start-Process PowerShell.exe -ArgumentList ("-c & '$scriptFullName' -Service")
}

# Ref 765fb12
function RMKSchedulerSCMStop {
  # Ref 6e3aaf: Param -SCMStop
  # The SCM tells us to STOP the service
  # Do whatever is necessary to stop the service script instance
  Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1003 -EntryType Information -Message "$scriptName -SCMStop: Stopping script $scriptName -Service"
  LogInfo "$scriptName -SCMStop: Stopping script $scriptName -Service"
  # Send an exit message to the service instance
  # Ref 3399b1
  LogDebug "$scriptName -SCMStop: 'Send-PipeMessage $RMKSchedulerPipeName exit'"
  Send-PipeMessage $RMKSchedulerPipeName "exit"
}

# Ref bba3224
function RMKSchedulerService {
  # Ref 8b0f1a: Param -Service
  Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1005 -EntryType Information -Message "$scriptName -Service # Beginning background job"
  try {
    # Start the control pipe handler thread
    # !! TO AVOID NASTY ERROR MESSAGES WHILE DEBUGGING ABOUT PIPE HANDLER THREADS,
    # execute this before you srtop the debugger: "Get-PSThread | Remove-PSThread"
    # This frees up the pipe handler thread.
    $pipeThread = Start-PipeHandlerThread $RMKSchedulerPipeName -Event "ControlMessage"

    # Lastly, as we are now within the service, call myself again with -Run to execute the scheduler.
    # ref 9177b1b
    $process = Start-Process Powershell.exe -ArgumentList "-File", "$scriptFullName", "-Run" -NoNewWindow -PassThru

    # After the scheduler has been started, we immediately come here (we do not wait),
    # because we must listen for "exit" events coming from RobotmkScheduler.ps1 via Command Pipe
    do {
      # Keep running until told to exit by the -Stop handler
      $evt = Wait-Event # Wait for the next incoming event
      $source = $evt.SourceIdentifier
      $message = $evt.MessageData
      $eventTime = $evt.TimeGenerated.TimeofDay
      LogDebug "Received an event at $eventTime from ${source}: $message"
      $evt | Remove-Event # Flush the event from the queue
      switch ($message) {
        "ControlMessage" {
          # Required. Message received by the control pipe thread
          $state = $evt.SourceEventArgs.InvocationStateInfo.state
          LogDebug "$script -Service # Thread $source state changed to $state"
          switch ($state) {
            "Completed" {
              $message = Receive-PipeHandlerThread $pipeThread
              LogInfo "$scriptName -Service # Received control message: $Message"
              # Ref 3399b1
              if ($message -eq "exit") {
                # Terminate the background job and exit the loop
                # TODO: Force quit?
                $job | Stop-Job
                break
              }
            }
            "Failed" {
              $threaderror = Receive-PipeHandlerThread $pipeThread
              LogInfo "$scriptName -Service # $source thread failed: $threaderror"
              Start-Sleep 1 # Avoid getting too many errors
              $pipeThread = Start-PipeHandlerThread $RMKSchedulerPipeName -Event "ControlMessage" # Retry
            }
          }
        }
        default {
          # Should not happen
          LogInfo "$scriptName -Service # Unexpected event from ${source}: $Message"
        }
      }
    } while ($true)  # Message receive loop
  }
  catch {
    # An exception occurred while runnning the service
    $msg = $_.Exception.Message
    $line = $_.InvocationInfo.ScriptLineNumber
    LogInfo "$scriptName -Service # Error at line ${line}: $msg"
  }
  finally {
    # Invoked in all cases: Exception or normally by -Stop
    # if process is still running, kill it
    if ($process.HasExited -eq $false) {
      Stop-Process -Id $process.Id
    }

    # Remove controller deadman switch - should terminate the scheduler soon
    Remove-Item $controller_deadman_file -ErrorAction SilentlyContinue
    # Terminate the control pipe handler thread and cleanup any remaining threads
    if ($pipeThread -ne $null) {
      Remove-PipeHandlerThread $pipeThread
    }
    Get-PSThread | Remove-PSThread # Remove all remaining threads
    # Flush all leftover events (There may be some that arrived after we exited the while event loop, but before we unregistered the events)
    $events = Get-Event | Remove-Event
    # Log a termination event, no matter what the cause is.
    Write-EventLog -LogName $WinEventLog -Source $RMKSchedulerServiceName -EventId 1006 -EntryType Information -Message "$script -Service # Exiting"
    LogInfo "$scriptName -Service # Exiting"
  }
  return
}

# Ref 3c3c3c3 (Tester)
# Ref 4d4d4d4 (Runner)
# Ref 9177b1b
function RMKScheduler {

  LogDebug "Entering RobotmkScheduler Main Routine. Checking for Rcc..."
  # This is the main Routine for the Robotmk Scheduler.
  if (RCCIsAvailable) {
    $blueprint = GetCondaBlueprint $conda_yml

    if ( IsRCCEnvReady $blueprint) {
      # if the RCC environment is ready, start the Scheduler if not yet running
      LogInfo "Robotmk RCC environment is ready to use."
      if (IsSchedulerRunning) {
        LogInfo "Nothing to do, Robotmk Scheduler is already running."
        return
      }
    }
    else {
      # otherwise, try to create the environment
      LogWarn "RCC environment is NOT (yet) ready to use; must create a new one."
      CreateRCCEnvironment $blueprint
    }
    # Run, Scheduler! (Starts the RCC task "scheduler")
    # Ref 9177b1b
    RunRobotmkTask "scheduler"
  }
  else {
    # TODO: If no RCC, use native python execution
    $Binary = $PythonExe
    $Arguments = "$PythonExe $RobotmkScheduler"
  }



}


# Ref 5887a1
function ResetRMKSchedulerService {
  # There are two situations when a environment need tabula rasa:
  # 1. The PS script in /plugins is newer than the Service script.
  $scheduler_script_needs_update = RMKSchedulerServiceScriptNeedsUpdate
  # 2. If RCC is used: Monitor detects that there is a newer conda.yml file than the one used in the RUNNING RCC env in use.
  $rcc_env_needs_update = RCCEnvNeedsUpdate

  if ($scheduler_script_needs_update -or $rcc_env_needs_update) {
    LogInfo "Updating the Scheduler Service..."
    RMKSchedulerStop
    if ($scheduler_script_needs_update) {
      RMKSchedulerRemove
      RMKSchedulerInstall
    }
    RMKSchedulerStart
    # If RCC present, save current conda.yaml hash to cache file to prevent subsequent runs don't see a difference anymore.
    SaveCondaFileHash $condahash_yml
  }


}


function Invoke-Process {
  <#
	.GUID b787dc5d-8d11-45e9-aeef-5cf3a1f690de
	.AUTHOR Adam Bertram
	.COMPANYNAME Adam the Automator, LLC
	.TAGS Processes
	#>
  [CmdletBinding(SupportsShouldProcess)]
  param
  (
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string]$FilePath,

    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$ArgumentList
  )

  $ErrorActionPreference = 'Stop'

  try {
    $stdOutTempFile = "$env:TEMP\$((New-Guid).Guid)"
    $stdErrTempFile = "$env:TEMP\$((New-Guid).Guid)"

    $startProcessParams = @{
      FilePath               = $FilePath
      ArgumentList           = $ArgumentList
      RedirectStandardError  = $stdErrTempFile
      RedirectStandardOutput = $stdOutTempFile
      Wait                   = $true;
      PassThru               = $true;
      NoNewWindow            = $true;
    }
    if ($PSCmdlet.ShouldProcess("Process [$($FilePath)]", "Run with args: [$($ArgumentList)]")) {
      $cmd = Start-Process @startProcessParams
      $cmdOutput = Get-Content -Path $stdOutTempFile -Raw
      $cmdError = Get-Content -Path $stdErrTempFile -Raw
      return @{
        ExitCode = $cmd.ExitCode
        Stdout   = $cmdOutput
        Stderr   = $cmdError
        Output   = $cmdOutput + $cmdError
      }
    }
  }
  catch {
    $PSCmdlet.ThrowTerminatingError($_)
  }
  finally {
    Remove-Item -Path $stdOutTempFile, $stdErrTempFile -Force -ErrorAction Ignore
  }
}




function IsRobotmkSchedulerServiceRunning {
  # Check if Service is running
  $service = Get-Service -Name $RMKSchedulerServiceName -ErrorAction SilentlyContinue
  if ($service -eq $null) {
    #LogInfo "Service $RMKSchedulerServiceName not installed."
    return $false
  }
  else {
    if ($service.Status -eq "Running") {
      #LogDebug "Service $RMKSchedulerServiceName running."
      return $true
    }
    else {
      #LogInfo "Service $RMKSchedulerServiceName not running."
      return $false
    }
  }
}

function StartRobotmkSchedulerService {
  # Check if Service is running
  $service = Get-Service -Name $RMKSchedulerServiceName -ErrorAction SilentlyContinue
  if ($service -eq $null) {
    LogInfo "Service $RMKSchedulerServiceName not installed."
    return $false
  }
  else {
    if ($service.Status -eq "Running") {
      LogDebug "Service $RMKSchedulerServiceName already running."
      return $true
    }
    else {
      LogInfo "Starting service $RMKSchedulerServiceName."
      Start-Service -Name $RMKSchedulerServiceName -ErrorAction SilentlyContinue
      return $true
    }
  }
}




function IsSchedulerRunning {
  # TODO: command string must be updated, has changed meanwhile
  $processes = GetProcesses -Cmdline "%robotmk.exe agent scheduler"
  # if length of array is 0, no process is running
  if ($processes.Length -eq 0) {
    if (Test-Path $scheduler_pidfile) {
      LogInfo "No process 'robotmk.exe agent scheduler' is running, removing stale PID file $scheduler_pidfile."
      Remove-Item $scheduler_pidfile -Force -ErrorAction SilentlyContinue
    }
    else {
      LogDebug "No process 'robotmk.exe agent scheduler' is running."
    }
    return $false
  }
  else {
    # Read PID from pidfile and check if THIS is still running
    # - PID from file is found: OK, go out
    # - PID from file not found: delete deadman file (forces the to also exit)
    if (Test-path $scheduler_pidfile) {
      $pidfromfile = Get-Content $scheduler_pidfile
      # if pidfromfile is in the list of running processes, we are good
      if ($processes -contains $pidfromfile) {
        LogDebug "The PID $pidfromfile is already running and in pidfile $scheduler_pidfile."
        return $true
      }
      else {
        LogError "The PID read from $scheduler_pidfile ($pidfromfile) des NOT seem to run."
        # option 1: kill all processes (favoured)
        LogWarn "Killing all processes matching the pattern '*robotmk*agent*(fg/bg)': $processes"
        $processes | ForEach-Object {
          Stop-Process -Id $_ -Force
        }
        # option 2: only remove the deadman file (py-Robotmk will exit; use this only if killing os not an option)
        #Remove-Item $controller_deadman_file -Force -ErrorAction SilentlyContinue
        return $false
      }
    }
    else {
      LogWarn "Processes matching the pattern '*robotmk*agent*(fg/bg)' are running ($processes), but NO PID file found for."
      LogWarn "Waiting for scheduler to create a PID file ($file) itself."
    }
  }

}


#   _____   _____ _____   _          _
#  |  __ \ / ____/ ____| | |        | |
#  | |__) | |   | |      | |__   ___| |_ __   ___ _ __
#  |  _  /| |   | |      | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | \ \| |___| |____  | | | |  __/ | |_) |  __/ |
#  |_|  \_\\_____\_____| |_| |_|\___|_| .__/ \___|_|
#                                     | |
#                                     |_|

function SaveCondaFileHash {
  # Get hash of conda.yaml and store it in conda.yaml.hash
  Param(
    [Parameter(Mandatory = $false)]
    [string]$conda_yml_hash
  )
  if ((AssertFileExists $conda_yml) -And (Test-Path $RCCExe)) {
    if (-Not($conda_yml_hash)) {
      $conda_yml_hash = CalculateCondaFilehash
    }
    LogDebug "Storing conda.yaml hash $conda_yml_hash in ${conda_yml}.hash."
    $conda_yml_hash | Out-File $conda_yml_hashfile -Force
  }
}

function CalculateCondaFilehash {
  # Calculate a hash of conda.yaml
  if (AssertFileExists $conda_yml) {
    $conda_hash = Get-FileHash $conda_yml
    return $conda_hash.Hash.Substring(0, 8)
  }
  else {
    return 00000000
  }
}
function ReadCondaFilehash {
  # Read hash of conda.yaml from conda.yaml.hash
  if (AssertFileExists $conda_yml_hashfile) {
    $conda_hash = Get-Content $conda_yml_hashfile
    return $conda_hash
  }
  else {
    return "00000000"
  }
}

function IsRCCEnvReady {
  # This approach checks if the blueprint is really in the catalog list.
  param (
    [Parameter(Mandatory = $True)]
    [string]$blueprint
  )

  if ((RCCCatalogContainsBlueprint $blueprint) -and (RCCHolotreeHasSpacesForBlueprint $blueprint)) {
    if (IsFlagfilePresent $Flagfile_rcc_robotmk_env_created) {
      return $true
    }
    else {
      TouchFile $Flagfile_rcc_robotmk_env_created	 "RCC env ready flagfile"
      return $true
    }
  }
  else {
    RemoveFlagfile $Flagfile_rcc_robotmk_env_created
    return $false
  }
}

function GetCondaBlueprint {
  # Get the blueprint hash for conda.yaml
  param (
    [Parameter(Mandatory = $True)]
    [string]$conda_yaml
  )
  LogDebug "Calculating blueprint hash for $conda_yaml..."
  if (-Not(Test-Path $conda_yaml)) {
    LogError "File $conda_yaml does not exist."
    exit 1
  }
  else {
    LogDebug "!!  rcc ht hash $conda_yaml"
    try {
      $ret = Invoke-Process -FilePath $RCCExe -ArgumentList "ht hash $conda_yaml"
      $out = $ret.Output
      LogDebug $out
      #$condahash = & $RCCExe ht hash $conda_yaml 2>&1
      $m = $out -match "Blueprint hash for.*is (?<blueprint>[A-Za-z0-9]*)\."
      $blueprint = $Matches.blueprint
      return $blueprint
    }
    catch {
      LogError "Error while calculating blueprint hash for ${conda_yaml}: $($_.Exception.Message)"
      exit 1
    }
  }
}

function RCCCatalogContainsBlueprint {
  param (
    [Parameter(Mandatory = $True)]
    [string]$blueprint
  )
  LogDebug "Checking if blueprint $blueprint is in RCC catalog..."
  LogDebug "!!  rcc ht catalogs"
  $ret = Invoke-Process -FilePath $RCCExe -ArgumentList "ht catalogs"
  $rcc_catalogs = $ret.Output
  #	$rcc_catalogs = & $RCCExe ht catalogs 2>&1
  #$catalogstring = [string]::Concat($rcc_catalogs)
  $catalogstring = $rcc_catalogs -join "\n"
  LogDebug "Catalogs:\n $rcc_catalogs"
  if ($catalogstring -match "$blueprint") {
    LogDebug "OK: Blueprint $blueprint is in RCC catalog."
    return $true
  }
  else {
    LogWarn "Blueprint $blueprint is NOT in RCC catalog."
    return $false
  }
}

function RCCHolotreeHasSpacesForBlueprint {
  # Checks if the RCC holotree spaces contain BOTH a line for SCHEDULER and OUTPUT space
  param (
    [Parameter(Mandatory = $True)]
    [string]$blueprint
  )
  # Example: rcc.robotmk  output  c939e5d2d8b335f9
  LogDebug "Checking if holotree spaces contain both, a line for SCHEDULER and OUTPUT space..."
  LogDebug "!!  rcc ht list"
  $ret = Invoke-Process -FilePath $RCCExe -ArgumentList "ht list"
  $holotree_spaces = $ret.Output
  $spaces_string = $holotree_spaces -join "\n"
  LogDebug "Holotree spaces: \n$spaces_string"
  # SCHEDULER SPACE
  $scheduler_space_found = ($spaces_string -match "rcc.$rcc_robotmk_controller\s+$rcc_robotmk_space_scheduler\s+$blueprint")
  if (-Not ($scheduler_space_found)) {
    LogWarn "Conda hash '$blueprint' not found for holotree space 'rcc.$rcc_robotmk_controller/$rcc_robotmk_space_scheduler'."
  }
  else {
    LogDebug "OK: Conda hash '$blueprint' found for holotree space 'rcc.$rcc_robotmk_controller/$rcc_robotmk_space_scheduler'."
  }
  # OUTPUT SPACE
  $output_space_found = ($spaces_string -match "rcc.$rcc_robotmk_controller\s+$rcc_robotmk_space_output\s+$blueprint")
  if (-Not ($output_space_found)) {
    LogWarn "Conda hash '$blueprint' not found for holotree space 'rcc.$rcc_robotmk_controller/$rcc_robotmk_space_output'."
  }
  else {
    LogDebug "OK: Conda hash '$blueprint' found for holotree space 'rcc.$rcc_robotmk_controller/$rcc_robotmk_space_output'."
  }

  if ($scheduler_space_found -and $output_space_found) {
    return $true
  }
  else {
    return $false
  }
}


function RCCEnvironmentCreate {
  # Creates/Ensures an environment with controller (app) and space (mode)
  param (
    [Parameter(Mandatory = $True)]
    [string]$robot_yml,
    [Parameter(Mandatory = $True)]
    [string]$controller,
    [Parameter(Mandatory = $True)]
    [string]$space
  )
  LogInfo "Creating Holotree space '$controller/$space'."
  $Arguments = "holotree vars --controller $controller --space $space -r $robot_yml"
  LogInfo "!!  $RCCExe $Arguments"
  $ret = Invoke-Process -FilePath $RCCExe -ArgumentList $Arguments
  $rc = $ret.ExitCode
  LogDebug $ret.Output
  if ($rc -eq 0) {
    LogInfo "RCC environment creation for Robotmk successful."
  }
  else {
    LogError "RCC environment creation for Robotmk FAILED for some reason."
  }
}

function RCCImportHololib {
  # Runs a RCC task
  param (
    [Parameter(Mandatory = $True)]
    [string]$hololib_zip
  )
  $Arguments = "holotree import $hololib_zip"
  $p = Start-Process -Wait -FilePath $RCCExe -ArgumentList $Arguments -NoNewWindow
  $p.StandardOutput
  $p.StandardError
}

function RCCDisableTelemetry {
  LogDebug "Disabling RCC telemetry..."
  & $RCCExe "configure", "identity", "--do-not-track"
}

function RCCEnvNeedsUpdate {
  # If RCC present, this function compares the conda.yaml file hash with the cached one.
  # No change/no RCC: return $False
  # Change, Rese was needed: return $True
  # Used in Ref 5887a1
  if (Test-Path $RCCExe) {

    if (IsFlagfileYoungerThanMinutes $Flagfile_rcc_robotmk_env_creation_in_progress $RCC_robotmk_env_max_creation_minutes) {
      LogInfo "Another Robotmk RCC environment creation is in progress (flagfile $Flagfile_rcc_robotmk_env_creation_in_progress present and younger than $RCC_robotmk_env_max_creation_minutes min)."
    }

    $condahash_yml = CalculateCondaFilehash
    $condahash_cache = ReadCondaFilehash
    if ($condahash_yml -eq $condahash_cache) {
      LogDebug "conda.yaml hash is unchanged ($condahash_cache), current RCC environment is up to date."
      return $False
    }
    else {
      LogInfo "conda.yaml hash has changed ($condahash_cache -> $condahash_yml). Current RCC environment needs to be updated."
      return $True
    }
  }
  else {
    return $False
  }

}


function RCCIsAvailable {
  # Check if the RCCExe binary is present. If not, download it.
  if (Test-Path $RCCExe) {
    LogDebug "RCC.exe found at $RCCExe."
    RCCDisableTelemetry
    return $true
  }
  else {
    # TODO: Downloading RCC is only for convenience. It should be removed finally.
    LogInfo "RCCExe $RCCExe not found, downloading it."
    $RCCExeUrl = "https://downloads.robocorp.com/rcc/releases/v11.30.0/windows64/rcc.exe"
    [Net.ServicePointManager]::SecurityProtocol = "tls12, tls11, tls"
    Invoke-WebRequest -Uri $RCCExeUrl -OutFile $RCCExe
    if (Test-Path $RCCExe) {
      LogDebug "RCC.exe downloaded to $RCCExe and available."
      return $true
    }
    else {
      LogError "RCC.exe could not be downloaded to $RCCExe."
      return $false
    }
  }
}

function CreateRCCEnvironment {
  # Creates a RCC environment for Robotmk
  Param (
    [Parameter(Mandatory = $True)]
    [string]$blueprint
  )
  RemoveFlagfile $Flagfile_rcc_robotmk_env_created
  TouchFile $Flagfile_rcc_robotmk_env_creation_in_progress "RCC creation state file"
  if (Test-Path ($hololib_zip)) {
    LogInfo "$hololib_zip found, importing it"
    RCCImportHololib "$hololib_zip"
    # TODO: after import, create spaces for scheduler and output
  }
  else {
    LogInfo "No hololib found for this environment; creating from sources..."
    # Create a separate Holotree Space for scheduler and output
    RCCEnvironmentCreate $robot_yml $rcc_robotmk_controller $rcc_robotmk_space_scheduler
    RCCEnvironmentCreate $robot_yml $rcc_robotmk_controller $rcc_robotmk_space_output
  }
  # This takes some minutes...
  # Watch the progress with `rcc ht list` and `rcc ht catalogs`. First the catalog is created, then
  # both spaces.
  if (RCCCatalogContainsBlueprint $blueprint) {
    TouchFile $Flagfile_rcc_robotmk_env_created "RCC env ready flagfile"
    RemoveFlagfile $Flagfile_rcc_robotmk_env_creation_in_progress
    LogInfo "OK: Environments for Robotmk created and ready to use."
  }
  else {
    LogInfo "RCC environment creation for Robotmk failed for some reason. Exiting."
    RemoveFlagfile $Flagfile_rcc_robotmk_env_creation_in_progress
  }
}

# Ref 9177b1b
function RunRobotmkTask {
  param (
    [Parameter(Mandatory = $True)]
    [string]$rmkmode
  )
  $rcctask = "agent-$rmkmode"

  # Determine which holotree space to use (scheduler/output)
  $space = (Get-Variable -Name "rcc_robotmk_space_$rmkmode").Value
  LogDebug "Running Robotmk task '$rcctask' in Holotree space '$rcc_robotmk_controller/$space'"
  $Arguments = "task run --controller $rcc_robotmk_controller --space $space -t $rcctask -r $robot_yml"
  LogDebug "!!  $RCCExe $Arguments"
  $ret = Invoke-Process -FilePath $RCCExe -ArgumentList $Arguments
  # -----------------------------------------
  # --- ROBOTMK SCHEDULER IS RUNNING HERE ---
  # --------------- DAEMONIZED --------------
  # -----------------------------------------
  # TODO: Use this generic function to start both scheduler and output (which returns!)

  # We reach this point when the RCC task has been terminated.
  $rc = $ret.ExitCode
  # Read last exit code from file (RCC cannot return the exit code of the task. )
  $robotmk_scheduler_lastexitcode = GetLastSchedulerExitCode

  LogInfo "Robotmk task '$rcctask' terminated."
  LogInfo "Last Message was: '$robotmk_scheduler_lastexitcode'"
}



#   _          _
#  | |        | |
#  | |__   ___| |_ __   ___ _ __
#  | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | | |  __/ | |_) |  __/ |
#  |_| |_|\___|_| .__/ \___|_|
#               | |
#               |_|

function GetLastSchedulerExitCode {
  # Returns from file the last exit code of the Robotmk Scheduler
  if (Test-Path $robotmk_scheduler_lastexitcode) {
    $content = Get-Content $robotmk_scheduler_lastexitcode
  }
  else {
    $content = "- Robotmk Scheduler did not write any exit code (file does not exist)"
  }
  return $content
}

function AssertFileExists {
  param (
    [Parameter(Mandatory = $True)]
    [string]$path,
    [Parameter(Mandatory = $False)]
    [string]$name = "file"
  )
  if (Test-Path $path) {
    #LogDebug "OK: $name '$path' found."
    return $true
  }
  else {
    LogError "ERROR: $name '$path' not found."
    return $false
  }
}

function TouchFile {
  param (
    [Parameter(Mandatory = $True)]
    [string]$path,
    [Parameter(Mandatory = $False)]
    [string]$name = "file"
  )
  LogDebug "Touching $name $path"
  $nul > $path
}

function RemoveFlagfile {
  param (
    [Parameter(Mandatory = $True)]
    [string]$path = $null
  )
  LogDebug "Removing flagfile $path"
  Remove-Item ($path) -Force -ErrorAction SilentlyContinue
}


function IsFlagfilePresent {
  param (
    [Parameter(Mandatory = $True)]
    [string]$flagfile
  )
  if (Test-Path $flagfile) {
    LogInfo "Flagfile $flagfile found"
    return $true
  }
  else {
    return $false
  }
}


# function to reads file's timestamp and return true if file is younger than 60 seconds
function IsFlagfileYoungerThanMinutes {
  param (
    [Parameter(Mandatory = $True)]
    [string]$path,
    [Parameter(Mandatory = $True)]
    [int]$minutes
  )
  # exit if file does not exist
  if (Test-Path $path) {
    $now = Get-Date
    $lastexec = Get-Date (Get-Item $path).LastWriteTime
    $diff = $now - $lastexec
    if (($diff.TotalSeconds / 60) -lt $minutes) {
      return $true
    }
    else {
      return $false
    }
  }
  else {
    return $false
  }
}

function GetProcesses {
  param (
    [Parameter(Mandatory = $True)]
    [string]$Cmdline
  )
  $processId = Get-WmiObject -Query "SELECT * FROM Win32_Process WHERE CommandLine like '$Cmdline'" | Select ProcessId

  #LogInfo "ProcessId: $processId"
  return $processId.processId

}

function IsProcessRunning {
  Param(
    [Parameter(Mandatory = $true)]
    [string]$Cmdline
  )
  $processes = GetProcesses -Cmdline "$Cmdline"
  # if length of array is 0, no process is running
  if ($processes.Length -eq 0) {
    return $false
  }
  else {
    return $true
  }
}

function KillProcessByCmdline {
  Param(
    [Parameter(Mandatory = $true)]
    [string]$Cmdline
  )
  $processes = GetProcesses -Cmdline "$Cmdline"
  # if length of array is 0, no process is running
  if ($processes.Length -gt 0) {
    foreach ($process in $processes) {
      LogInfo "Killing process with command line '$Cmdline' (PID $process)"
      Stop-Process -Id $process -Force
    }
  }
}

function Ensure-Directory {
  param (
    [Parameter(Mandatory = $True)]
    [string]$directory
  )
  if (-Not (Test-Path $directory)) {
    #LogInfo "Directory $directory does not exist, creating it."
    New-Item -ItemType Directory -Path $directory -Force -ErrorAction SilentlyContinue | Out-Null
  }
}

function Get-EnvVar {
  # return environment variable or default value
  param (
    [Parameter(Mandatory = $True)]
    [string]$name,
    [Parameter(Mandatory = $True)]
    [string]$default = ""
  )
  $value = [System.Environment]::GetEnvironmentVariable($name)
  if ($value -eq $null) {
    return $default
  }
  else {
    return $value
  }
}

function Set-EnvVar {
  param (
    [Parameter(Mandatory = $True)]
    [string]$name,
    [Parameter(Mandatory = $True)]
    [string]$value
  )
  [System.Environment]::SetEnvironmentVariable($name, $value)
}

function Get-CurrentUserName {
  # Identify the user name. We use that for logging.
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $currentUserName = $identity.Name # Ex: "NT AUTHORITY\SYSTEM" or "Domain\Administrator"
  LogDebug "CurrentUsername: $currentUserName"
  return $currentUserName
}



Function Now {
  Param (
    [Switch]$ms, # Append milliseconds
    [Switch]$ns         # Append nanoseconds
  )
  $Date = Get-Date
  $now = ""
  $now += "{0:0000}-{1:00}-{2:00} " -f $Date.Year, $Date.Month, $Date.Day
  $now += "{0:00}:{1:00}:{2:00}" -f $Date.Hour, $Date.Minute, $Date.Second
  $nsSuffix = ""
  if ($ns) {
    if ("$($Date.TimeOfDay)" -match "\.\d\d\d\d\d\d") {
      $now += $matches[0]
      $ms = $false
    }
    else {
      $ms = $true
      $nsSuffix = "000"
    }
  }
  if ($ms) {
    $now += ".{0:000}$nsSuffix" -f $Date.MilliSecond
  }
  return $now
}


function SetScriptVars {
  # TODO: if "robocorp_home" is set, use that as base dir
  $Global:CMKAgentSection = "<<<robotmk:sep(0)>>>"
  $Global:SubsecController = "[[[robotmk-ctrl]]]"


  # Programdata var
  $ProgramData = [System.Environment]::GetFolderPath("CommonApplicationData")
  $Global:PDataCMK = "$ProgramData\checkmk"

  # The name of the Checkmk Agent Plugin
  $Global:RMK_Controller = "robotmk-ctrl"
  $Global:RMK_ControllerName = "${RMK_Controller}.ps1"

  # Windows Service vars

  $Global:RMKSchedulerServiceName = "RobotmkScheduler"
  $Global:RMKSchedulerServiceDisplayName = $RMKSchedulerServiceName
  $Global:RMKSchedulerServiceDescription = "RobotmkScheduler is a side agent of Checkmk Agent. It runs Robot Framework suites asynchronously and generates the required Python environments via RCC."
  $Global:RMKSchedulerServiceStartupType = "Manual"
  $Global:RMKSchedulerServiceDependsOn = @("CheckmkService")

  # Windows service executable vars
  $Global:RMKSchedulerInstallDir = "${PDataCMK}\robotmk"
  $Global:RMKScheduler = $RMKSchedulerServiceName
  $Global:RMKSchedulerName = "${RMKScheduler}.ps1"
  $Global:RMKSchedulerFullName = "$RMKSchedulerInstallDir\${RMKSchedulerName}"
  $Global:RMKSchedulerFullNameEscaped = $RMKSchedulerFullName -replace "\\", "\\"


  # Where to install the service files
  $Global:RMKSchedulerExeName = "$RMKSchedulerServiceName.exe"
  $Global:RMKSchedulerExeFullName = "$RMKSchedulerInstallDir\$RMKSchedulerExeName"
  $Global:RMKSchedulerPipeName = "Service_$RMKSchedulerServiceName"

  # if there is a varfile, read these ROBOTMK vars into env
  ReadRMKVars

  if ($env:ROBOTMK_COMMON_path__prefix) {
    # DEVELOPER ZONE ===
    # For debugging and development
    $PDataCMKAgent = $env:ROBOTMK_COMMON_path__prefix
  }
  else {
    # PRODUCTION ZONE ===
    # Default c:/ProgramData/checkmk/agent/
    $PDataCMKAgent = "$PDataCMK\agent"
  }
  $Global:RMKCfgDir = "$PDataCMKAgent\config\robotmk"
  $Global:RMKLogDir = "$PDataCMKAgent\log\robotmk"
  $Global:RMKTmpDir = "$PDataCMKAgent\tmp\robotmk"


  $Global:RMKLogfile = "$RMKLogDir\${script}.log"

  $Global:WinEventLog = "Application"
  $Global:ROBOCORP_HOME = if ($env:ROBOCORP_HOME) { $env:ROBOCORP_HOME } else {
    # use user tmp dir if not set
    # TODO: read this from robotmk.yml?
    $env:TEMP + "\ROBOCORP"
  };
  Set-EnvVar "ROBOCORP_HOME" $ROBOCORP_HOME

  # FILES ========================================

  $Global:RCCExe = $PDataCMKAgent + "\bin\rcc.exe"
  $Global:scheduler_pidfile = $RMKTmpDir + "\robotmk_scheduler.pid"

  $Global:robot_yml = $RMKCfgDir + "\robot.yaml"
  $Global:conda_yml = $RMKCfgDir + "\conda.yaml"
  $Global:conda_yml_hashfile = $RMKTmpDir + "\robotmk_conda_yml_hash.txt"
  $Global:hololib_zip = $RMKCfgDir + "\hololib.zip"
  $Global:controller_deadman_file = $RMKTmpDir + "\robotmk_controller_deadman_file"
  # This flagfile indicates that both there is a usable holotree space for "robotmk agent/output"
  $Global:Flagfile_rcc_robotmk_env_created = $RMKTmpDir + "\rcc_robotmk_env_created"
  $Global:robotmk_scheduler_lastexitcode = $RMKTmpDir + "\robotmk_scheduler_lastexitcode"
  # IMPORTANT! All other Robot subprocesses must respect this file and not start if it is present!
  # (There is only ONE RCC creation allowed at a time.)
  $Global:Flagfile_rcc_robotmk_env_creation_in_progress = $RMKTmpDir + "\rcc_robotmk_env_creation_in_progress.lock"
  # how many minutes to wait for a/any single RCC env creation to be finished (maxage of $Flagfile_rcc_robotmk_env_creation_in_progress)
  $Global:RCC_robotmk_env_max_creation_minutes = 1

  # RCC namespaces make the RCC envs unique for each execution = suite run
  # see https://github.com/robocorp/rcc/blob/master/docs/recipes.md#how-to-control-holotree-environments
  # controller
  $Global:rcc_robotmk_controller = "robotmk"
  # - space for scheduler and output
  $Global:rcc_robotmk_space_scheduler = "scheduler"
  $Global:rcc_robotmk_space_output = "output"

  # C# stub code
  # ref 5f8dda
  $Global:source = @"
  using System;
  using System.ServiceProcess;
  using System.Diagnostics;
  using System.Runtime.InteropServices;                                 // SET STATUS
  using System.ComponentModel;                                          // SET STATUS

  public enum ServiceType : int {                                       // SET STATUS [
    SERVICE_WIN32_OWN_PROCESS = 0x00000010,
    SERVICE_WIN32_SHARE_PROCESS = 0x00000020,
  };                                                                    // SET STATUS ]

  public enum ServiceState : int {                                      // SET STATUS [
    SERVICE_STOPPED = 0x00000001,
    SERVICE_START_PENDING = 0x00000002,
    SERVICE_STOP_PENDING = 0x00000003,
    SERVICE_RUNNING = 0x00000004,
    SERVICE_CONTINUE_PENDING = 0x00000005,
    SERVICE_PAUSE_PENDING = 0x00000006,
    SERVICE_PAUSED = 0x00000007,
  };                                                                    // SET STATUS ]

  [StructLayout(LayoutKind.Sequential)]                                 // SET STATUS [
  public struct ServiceStatus {
    public ServiceType dwServiceType;
    public ServiceState dwCurrentState;
    public int dwControlsAccepted;
    public int dwWin32ExitCode;
    public int dwServiceSpecificExitCode;
    public int dwCheckPoint;
    public int dwWaitHint;
  };                                                                    // SET STATUS ]

  public enum Win32Error : int { // WIN32 errors that we may need to use
    NO_ERROR = 0,
    ERROR_APP_INIT_FAILURE = 575,
    ERROR_FATAL_APP_EXIT = 713,
    ERROR_SERVICE_NOT_ACTIVE = 1062,
    ERROR_EXCEPTION_IN_SERVICE = 1064,
    ERROR_SERVICE_SPECIFIC_ERROR = 1066,
    ERROR_PROCESS_ABORTED = 1067,
  };

  public class Service_$RMKSchedulerServiceName : ServiceBase { // $RMKSchedulerServiceName may begin with a digit; The class name must begin with a letter
    private System.Diagnostics.EventLog eventLog;                       // EVENT LOG
    private ServiceStatus serviceStatus;                                // SET STATUS

    public Service_$RMKSchedulerServiceName() {
      ServiceName = "$RMKSchedulerServiceName";
      CanStop = true;
      CanPauseAndContinue = false;
      AutoLog = true;

      eventLog = new System.Diagnostics.EventLog();                     // EVENT LOG [
      if (!System.Diagnostics.EventLog.SourceExists(ServiceName)) {
        System.Diagnostics.EventLog.CreateEventSource(ServiceName, "$WinEventLog");
      }
      eventLog.Source = ServiceName;
      eventLog.Log = "$WinEventLog";                                        // EVENT LOG ]
      EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName $RMKSchedulerServiceName()");      // EVENT LOG
    }

    [DllImport("advapi32.dll", SetLastError=true)]                      // SET STATUS
    private static extern bool SetServiceStatus(IntPtr handle, ref ServiceStatus serviceStatus);

    // 9833fa
    protected override void OnStart(string [] args) {
      EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStart() entrypoint. Now starting the scheduler service script '$RMKSchedulerFullNameEscaped' -SCMStart"); // EVENT LOG
      // Set the service state to Start Pending.                        // SET STATUS [
      // Only useful if the startup time is long. Not really necessary here for a 2s startup time.
      serviceStatus.dwServiceType = ServiceType.SERVICE_WIN32_OWN_PROCESS;
      serviceStatus.dwCurrentState = ServiceState.SERVICE_START_PENDING;
      serviceStatus.dwWin32ExitCode = 0;
      serviceStatus.dwWaitHint = 2000; // It takes about 2 seconds to start PowerShell
      SetServiceStatus(ServiceHandle, ref serviceStatus);               // SET STATUS ]
      // Start a child process with another copy of this script
      try {
        Process p = new Process();
        // Redirect the output stream of the child process.
        p.StartInfo.UseShellExecute = false;
        p.StartInfo.RedirectStandardOutput = true;
        p.StartInfo.FileName = "PowerShell.exe";
        p.StartInfo.Arguments = "-ExecutionPolicy Bypass -c & '$RMKSchedulerFullNameEscaped' -SCMStart"; // Works if path has spaces, but not if it contains ' quotes.
        p.Start();
        // Read the output stream first and then wait. (To avoid deadlocks says Microsoft!)
        string output = p.StandardOutput.ReadToEnd();
        // Wait for the completion of the script startup code, that launches the -Service instance
        p.WaitForExit();
        EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStart(): SCMStart came back with exit code " + p.ExitCode); // EVENT LOG
        if (p.ExitCode != 0) throw new Win32Exception((int)(Win32Error.ERROR_APP_INIT_FAILURE));
        // Success. Set the service state to Running.                   // SET STATUS
        serviceStatus.dwCurrentState = ServiceState.SERVICE_RUNNING;    // SET STATUS
      } catch (Exception e) {
        EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStart() // Failed to start $RMKSchedulerFullNameEscaped. " + e.Message, EventLogEntryType.Error); // EVENT LOG
        // Change the service state back to Stopped.                    // SET STATUS [
        serviceStatus.dwCurrentState = ServiceState.SERVICE_STOPPED;
        Win32Exception w32ex = e as Win32Exception; // Try getting the WIN32 error code
        if (w32ex == null) { // Not a Win32 exception, but maybe the inner one is...
          w32ex = e.InnerException as Win32Exception;
        }
        if (w32ex != null) {    // Report the actual WIN32 error
          serviceStatus.dwWin32ExitCode = w32ex.NativeErrorCode;
        } else {                // Make up a reasonable reason
          serviceStatus.dwWin32ExitCode = (int)(Win32Error.ERROR_APP_INIT_FAILURE);
        }                                                               // SET STATUS ]
      } finally {
        serviceStatus.dwWaitHint = 0;                                   // SET STATUS
        SetServiceStatus(ServiceHandle, ref serviceStatus);             // SET STATUS
        EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStart() // Exit"); // EVENT LOG
      }
    }

    protected override void OnStop() {
      EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStop() // Entry");   // EVENT LOG
      // Start a child process with another copy of ourselves
      try {
        Process p = new Process();
        // Redirect the output stream of the child process.
        p.StartInfo.UseShellExecute = false;
        p.StartInfo.RedirectStandardOutput = true;
        p.StartInfo.FileName = "PowerShell.exe";
        p.StartInfo.Arguments = "-ExecutionPolicy Bypass -c & '$RMKSchedulerFullNameEscaped' -SCMStop"; // Works if path has spaces, but not if it contains ' quotes.
        p.Start();
        // Read the output stream first and then wait. (To avoid deadlocks says Microsoft!)
        string output = p.StandardOutput.ReadToEnd();
        // Wait for the PowerShell script to be fully stopped.
        p.WaitForExit();
        if (p.ExitCode != 0) throw new Win32Exception((int)(Win32Error.ERROR_APP_INIT_FAILURE));
        // Success. Set the service state to Stopped.                   // SET STATUS
        serviceStatus.dwCurrentState = ServiceState.SERVICE_STOPPED;      // SET STATUS
      } catch (Exception e) {
        EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStop() // Failed to stop $RMKSchedulerFullNameEscaped. " + e.Message, EventLogEntryType.Error); // EVENT LOG
        // Change the service state back to Started.                    // SET STATUS [
        serviceStatus.dwCurrentState = ServiceState.SERVICE_RUNNING;
        Win32Exception w32ex = e as Win32Exception; // Try getting the WIN32 error code
        if (w32ex == null) { // Not a Win32 exception, but maybe the inner one is...
          w32ex = e.InnerException as Win32Exception;
        }
        if (w32ex != null) {    // Report the actual WIN32 error
          serviceStatus.dwWin32ExitCode = w32ex.NativeErrorCode;
        } else {                // Make up a reasonable reason
          serviceStatus.dwWin32ExitCode = (int)(Win32Error.ERROR_APP_INIT_FAILURE);
        }                                                               // SET STATUS ]
      } finally {
        serviceStatus.dwWaitHint = 0;                                   // SET STATUS
        SetServiceStatus(ServiceHandle, ref serviceStatus);             // SET STATUS
        EventLog.WriteEntry(ServiceName, "$RMKSchedulerExeName OnStop() // Exit"); // EVENT LOG
      }
    }

    public static void Main() {
      System.ServiceProcess.ServiceBase.Run(new Service_$RMKSchedulerServiceName());
    }
  }
"@
}


function WriteRMKVars {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Directory,
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$file
  )
  # Write all known ROBOTMK_ variables into a file (default: same location as the script).
  $RMKVars = Get-ChildItem -Path env: | Where-Object { $_.Name -like "ROBOTMK_*" }


  if ($RMKVars.Count -eq 0) {
    LogDebug "No variables to store."
  }
  else {
    LogInfo "- Writing $($RMKVars.Count) environment vars into ${file}.env"
    $content = $RMKVars | ForEach-Object { "$($_.Name)=$($_.Value)" }
    $file = "$Directory\${file}.env"
    $content | Out-File -FilePath $file -Encoding ascii
  }
}

function ReadRMKVars {
  # Checks if there is a varfile at the same location as the script, and if so, reads it into the environment.
  # The varfile is a PowerShell script that sets variables.
  $file = $scriptDir + "\" + $scriptVarfile
  if (Test-Path $file) {
    $fileContent = Get-Content -Path $file
    $fileContent | ForEach-Object {
      # ROBOTMK_var_name=foobar
      if ($_ -match "^\s*(?<varname>ROBOTMK_\w+)\s*=\s*(?<varvalue>.*)") {
        $varname = $Matches["varname"]
        $varvalue = $Matches["varvalue"]
        #LogDebug "Setting $varname=$varvalue"
        Set-Item -Path env:$varname -Value $varvalue
      }
    }
  }
}


#   _      ____   _____
#  | |    / __ \ / ____|
#  | |   | |  | | |  __
#  | |   | |  | | | |_ |
#  | |___| |__| | |__| |
#  |______\____/ \_____|




function Log {
  #[CmdletBinding()]
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Level,
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message,
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$file = "$RMKLogfile"
  )
  $LogTime = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffK")
  $PaddedLevel = $Level.PadRight(6)
  $mypid = $PID.ToString()
  $MsgArr = $Message.Split([System.Environment]::NewLine, [System.StringSplitOptions]::RemoveEmptyEntries)
  $pidstring = "[${mypid}]".PadRight(8)
  if ($script_arg) {
    $EXEC_PHASE = "$script_arg".PadRight(9)
  }
  else {
    $EXEC_PHASE = "".PadRight(9)
  }


  # if length of $MsgArr is more than 1, then we have a multiline message
  if ($MsgArr.Length -gt 1) {
    $prefix = "  |   "
  }
  else {
    $prefix = ""
  }
  $MsgArr | ForEach-Object { "$LogTime ${pidstring} ${EXEC_PHASE} ${PaddedLevel}  ${prefix}$_" >> "$file" }
  if (-Not ($RunningInBackground)) {
    $MsgArr | ForEach-Object { Write-Host "$LogTime ${pidstring} ${EXEC_PHASE} ${PaddedLevel}  ${prefix}$_" }
  }
  #"$logTime - $PadLevel ${PaddedPID} $Message" >> "$file"
}

function LogInfo {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message
  )
  Log "INFO" $Message
}

function LogDebug {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message
  )
  if ($DEBUG) {
    Log "DEBUG" $Message
  }

}

function LogError {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message
  )
  Log "ERROR" $Message
}

function LogWarn {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message
  )
  Log "WARN" $Message
}

function LogConfiguration {
  param(
    [Parameter()]
    [ValidateNotNullOrEmpty()]
    [string]$Message
  )
  LogDebug "--- 8< --------------------"
  LogDebug "CONFIGURATION:"
  LogDebug "- this script: $scriptFullName"
  LogDebug "- Scheduler Service Scriptname: $RMKScheduler"
  LogDebug "- PID: $PID"
  LogDebug "- RMKLogDir: $RMKLogDir"
  LogDebug "- RMKTmpDir: $RMKTmpDir"
  # TODO: for non-enterprise version, make this optional
  #LogDebug "- Use RCC: $UseRCC"
  #if ($UseRCC) {
  LogRCCConfig
  #}
  LogDebug "-------------------- >8 ---"
}

function LogRCCConfig {
  LogDebug "RCC CONFIGURATION:"
  LogDebug "- ROBOCORP_HOME: $ROBOCORP_HOME"
  LogDebug "- RCCEXE: $RCCEXE"
  LogDebug "- RMKCfgDir: $RMKCfgDir"
  LogDebug "- Robotmk RCC holotree spaces:"
  LogDebug "  - Robotmk agent scheduler: rcc.$rcc_robotmk_controller/$rcc_robotmk_space_scheduler"
  LogDebug "  - Robotmk agent output: rcc.$rcc_robotmk_controller/$rcc_robotmk_space_output"
}

function Write-ServiceStatus {
  $status = RMKSchedulerStatus
  Write-Host "$RMKSchedulerServiceName is $status"
}

#   _____   _____ _____ ______ _______      _______ _____ ______
#  |  __ \ / ____/ ____|  ____|  __ \ \    / /_   _/ ____|  ____|
#  | |__) | (___| (___ | |__  | |__) \ \  / /  | || |    | |__
#  |  ___/ \___ \\___ \|  __| |  _  / \ \/ /   | || |    |  __|
#  | |     ____) |___) | |____| | \ \  \  /   _| || |____| |____
#  |_|    |_____/_____/|______|_|  \_\  \/   |_____\_____|______|

# Inspired by
# https://msdn.microsoft.com/en-us/magazine/mt703436.aspx
# http://jf.larvoire.free.fr/progs/PSService.ps1

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Start-PSThread                                            #
#                                                                             #
#   Description     Start a new PowerShell thread                             #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#   Notes           Returns a thread description object.                      #
#                   The completion can be tested in $_.Handle.IsCompleted     #
#                   Alternative: Use a thread completion event.               #
#                                                                             #
#   References                                                                #
#    https://learn-powershell.net/tag/runspace/                               #
#    https://learn-powershell.net/2013/04/19/sharing-variables-and-live-objects-between-powershell-runspaces/
#    http://www.codeproject.com/Tips/895840/Multi-Threaded-PowerShell-Cookbook
#                                                                             #
#-----------------------------------------------------------------------------#

$PSThreadCount = 0              # Counter of PSThread IDs generated so far
$PSThreadList = @{}             # Existing PSThreads indexed by Id

Function Get-PSThread () {
  Param(
    [Parameter(Mandatory = $false, ValueFromPipeline = $true, Position = 0)]
    [int[]]$Id = $PSThreadList.Keys     # List of thread IDs
  )
  $Id | % { $PSThreadList.$_ }
}

Function Start-PSThread () {
  Param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ScriptBlock]$ScriptBlock, # The script block to run in a new thread
    [Parameter(Mandatory = $false)]
    [String]$Name = "", # Optional thread name. Default: "PSThread$Id"
    [Parameter(Mandatory = $false)]
    [String]$Event = "", # Optional thread completion event name. Default: None
    [Parameter(Mandatory = $false)]
    [Hashtable]$Variables = @{}, # Optional variables to copy into the script context.
    [Parameter(Mandatory = $false)]
    [String[]]$Functions = @(), # Optional functions to copy into the script context.
    [Parameter(Mandatory = $false)]
    [Object[]]$Arguments = @()          # Optional arguments to pass to the script.
  )

  $Id = $script:PSThreadCount
  $script:PSThreadCount += 1
  if (!$Name.Length) {
    $Name = "PSThread$Id"
  }
  $InitialSessionState = [System.Management.Automation.Runspaces.InitialSessionState]::CreateDefault()
  foreach ($VarName in $Variables.Keys) {
    # Copy the specified variables into the script initial context
    $value = $Variables.$VarName
    LogDebug "Adding variable $VarName=[$($Value.GetType())]$Value"
    $var = New-Object System.Management.Automation.Runspaces.SessionStateVariableEntry($VarName, $value, "")
    $InitialSessionState.Variables.Add($var)
  }
  foreach ($FuncName in $Functions) {
    # Copy the specified functions into the script initial context
    $Body = Get-Content function:$FuncName
    #LogDebug "Adding function $FuncName () {$Body}"
    LogDebug "Adding function $FuncName()"
    $func = New-Object System.Management.Automation.Runspaces.SessionStateFunctionEntry($FuncName, $Body)
    $InitialSessionState.Commands.Add($func)
  }
  $RunSpace = [RunspaceFactory]::CreateRunspace($InitialSessionState)
  $RunSpace.Open()
  $PSPipeline = [powershell]::Create()
  $PSPipeline.Runspace = $RunSpace
  $PSPipeline.AddScript($ScriptBlock) | Out-Null
  $Arguments | % {
    LogDebug "Adding argument [$($_.GetType())]'$_'"
    $PSPipeline.AddArgument($_) | Out-Null
  }
  $Handle = $PSPipeline.BeginInvoke() # Start executing the script
  if ($Event.Length) {
    # Do this after BeginInvoke(), to avoid getting the start event.
    Register-ObjectEvent $PSPipeline -EventName InvocationStateChanged -SourceIdentifier $Name -MessageData $Event
  }
  $PSThread = New-Object PSObject -Property @{
    Id         = $Id
    Name       = $Name
    Event      = $Event
    RunSpace   = $RunSpace
    PSPipeline = $PSPipeline
    Handle     = $Handle
  }     # Return the thread description variables
  $script:PSThreadList[$Id] = $PSThread
  $PSThread
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Receive-PSThread                                          #
#                                                                             #
#   Description     Get the result of a thread, and optionally clean it up    #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#-----------------------------------------------------------------------------#

Function Receive-PSThread () {
  [CmdletBinding()]
  Param(
    [Parameter(Mandatory = $false, ValueFromPipeline = $true, Position = 0)]
    [PSObject]$PSThread, # Thread descriptor object
    [Parameter(Mandatory = $false)]
    [Switch]$AutoRemove                 # If $True, remove the PSThread object
  )
  Process {
    if ($PSThread.Event -and $AutoRemove) {
      Unregister-Event -SourceIdentifier $PSThread.Name
      Get-Event -SourceIdentifier $PSThread.Name | Remove-Event # Flush remaining events
    }
    try {
      $PSThread.PSPipeline.EndInvoke($PSThread.Handle) # Output the thread pipeline output
    }
    catch {
      $_ # Output the thread pipeline error
    }
    if ($AutoRemove) {
      $PSThread.RunSpace.Close()
      $PSThread.PSPipeline.Dispose()
      $PSThreadList.Remove($PSThread.Id)
    }
  }
}

Function Remove-PSThread () {
  [CmdletBinding()]
  Param(
    [Parameter(Mandatory = $false, ValueFromPipeline = $true, Position = 0)]
    [PSObject]$PSThread                 # Thread descriptor object
  )
  Process {
    $_ | Receive-PSThread -AutoRemove | Out-Null
  }
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Send-PipeMessage                                          #
#                                                                             #
#   Description     Send a message to a named pipe                            #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#-----------------------------------------------------------------------------#

Function Send-PipeMessage () {
  Param(
    [Parameter(Mandatory = $true)]
    [String]$RMKSchedulerPipeName, # Named pipe name
    [Parameter(Mandatory = $true)]
    [String]$Message            # Message string
  )
  $PipeDir = [System.IO.Pipes.PipeDirection]::Out
  $PipeOpt = [System.IO.Pipes.PipeOptions]::Asynchronous

  $pipe = $null # Named pipe stream
  $sw = $null   # Stream Writer
  try {
    $pipe = new-object System.IO.Pipes.NamedPipeClientStream(".", $RMKSchedulerPipeName, $PipeDir, $PipeOpt)
    $sw = new-object System.IO.StreamWriter($pipe)
    $pipe.Connect(1000)
    if (!$pipe.IsConnected) {
      throw "Failed to connect client to pipe $RMKSchedulerPipeName"
    }
    $sw.AutoFlush = $true
    $sw.WriteLine($Message)
  }
  catch {
    LogError "Error sending pipe $RMKSchedulerPipeName message: $_"
  }
  finally {
    if ($sw) {
      $sw.Dispose() # Release resources
      $sw = $null   # Force the PowerShell garbage collector to delete the .net object
    }
    if ($pipe) {
      $pipe.Dispose() # Release resources
      $pipe = $null   # Force the PowerShell garbage collector to delete the .net object
    }
  }
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Receive-PipeMessage                                       #
#                                                                             #
#   Description     Wait for a message from a named pipe                      #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#   Notes           I tried keeping the pipe open between client connections, #
#                   but for some reason everytime the client closes his end   #
#                   of the pipe, this closes the server end as well.          #
#                   Any solution on how to fix this would make the code       #
#                   more efficient.                                           #
#-----------------------------------------------------------------------------#

Function Receive-PipeMessage () {
  Param(
    [Parameter(Mandatory = $true)]
    [String]$RMKSchedulerPipeName           # Named pipe name
  )
  $PipeDir = [System.IO.Pipes.PipeDirection]::In
  $PipeOpt = [System.IO.Pipes.PipeOptions]::Asynchronous
  $PipeMode = [System.IO.Pipes.PipeTransmissionMode]::Message

  try {
    $pipe = $null       # Named pipe stream
    $pipe = New-Object system.IO.Pipes.NamedPipeServerStream($RMKSchedulerPipeName, $PipeDir, 1, $PipeMode, $PipeOpt)
    $sr = $null         # Stream Reader
    $sr = new-object System.IO.StreamReader($pipe)
    $pipe.WaitForConnection()
    $Message = $sr.Readline()
    $Message
  }
  catch {
    LogError "Error receiving pipe message: $_"
  }
  finally {
    if ($sr) {
      $sr.Dispose() # Release resources
      $sr = $null   # Force the PowerShell garbage collector to delete the .net object
    }
    if ($pipe) {
      $pipe.Dispose() # Release resources
      $pipe = $null   # Force the PowerShell garbage collector to delete the .net object
    }
  }
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Start-PipeHandlerThread                                   #
#                                                                             #
#   Description     Start a new thread waiting for control messages on a pipe #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#   Notes           The pipe handler script uses function Receive-PipeMessage.#
#                   This function must be copied into the thread context.     #
#                                                                             #
#                   The other functions and variables copied into that thread #
#                   context are not strictly necessary, but are useful for    #
#                   debugging possible issues.                                #
#-----------------------------------------------------------------------------#

$pipeThreadName = "Control Pipe Handler"

Function Start-PipeHandlerThread () {
  Param(
    [Parameter(Mandatory = $true)]
    [String]$RMKSchedulerPipeName, # Named pipe name
    [Parameter(Mandatory = $false)]
    [String]$Event = "ControlMessage"   # Event message
  )
  $currentUserName = Get-CurrentUserName

  Start-PSThread -Variables @{  # Copy variables required by function Log() into the thread context
    logDir          = $RMKLogDir
    logFile         = $RMKLogfile
    currentUserName = $currentUserName
  } -Functions Now, Log, Receive-PipeMessage -ScriptBlock {
    Param($RMKSchedulerPipeName, $pipeThreadName)
    try {
      Receive-PipeMessage "$RMKSchedulerPipeName" # Blocks the thread until the next message is received from the pipe
    }
    catch {
      LogInfo "$pipeThreadName # Error: $_"
      throw $_ # Push the error back to the main thread
    }
  } -Name $pipeThreadName -Event $Event -Arguments $RMKSchedulerPipeName, $pipeThreadName
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        Receive-PipeHandlerThread                                 #
#                                                                             #
#   Description     Get what the pipe handler thread received                 #
#                                                                             #
#   Arguments       See the Param() block                                     #
#                                                                             #
#   Notes                                                                     #
#-----------------------------------------------------------------------------#

Function Receive-PipeHandlerThread () {
  Param(
    [Parameter(Mandatory = $true)]
    [PSObject]$pipeThread               # Thread descriptor
  )
  Receive-PSThread -PSThread $pipeThread -AutoRemove
}

#-----------------------------------------------------------------------------#
#                                                                             #
#   Function        $source                                                   #
#                                                                             #
#   Description     C# source of the PSService.exe stub                       #
#                                                                             #
#   Arguments                                                                 #
#                                                                             #
#   Notes           The lines commented with "SET STATUS" and "EVENT LOG" are #
#                   optional. (Or blocks between "// SET STATUS [" and        #
#                   "// SET STATUS ]" comments.)                              #
#                   SET STATUS lines are useful only for services with a long #
#                   startup time.                                             #
#                   EVENT LOG lines are useful for debugging the service.     #
#                                                                             #
#-----------------------------------------------------------------------------#



main
