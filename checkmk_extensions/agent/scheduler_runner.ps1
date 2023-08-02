# Enable StrictMode 3.0
Set-StrictMode -Version 3.0

class Config {
    [bool]$rcc

    # Constructor
    # This is a way to make the rcc property mandatory
    Config([bool]$rcc) {
        $this.rcc = $rcc
    }

    # Method to parse the config file (JSON) and set the value of rcc
    static [Config] ParseConfigFile($configFilePath) {

        $configFileContent = Get-Content -Raw -Path $configFilePath -ErrorAction Stop

        try {
            $configData = $configFileContent | ConvertFrom-Json -ErrorAction Stop
            # Assign the value from the config file to rcc
            $use_rcc = $configData.rcc -as [bool]
        }
        catch {
            throw $_.Exception.Message
        }

        return [Config]::new($use_rcc)
    }
}

class Command {
    [string]$Executable
    [System.Collections.Generic.List[string]]$ArgumentsList

    Command([string]$executable, [System.Collections.Generic.List[string]]$argumentsList) {
        $this.Executable = $executable
        $this.ArgumentsList = $argumentsList
    }

    # Method to create the command depending on the rcc value
    static [Command] CreateCommand([Config]$config) {
        $parentDir = Split-Path $PSScriptRoot -Parent

        if ($config.rcc) {
            $_executable = Join-Path $PSScriptRoot "rcc.exe"
            $robotPath = Join-Path $parentDir "config\robot.yaml"
            $_argumentsList = @("run", "--robot", $robotPath)
        }
        else {
            $_executable = (Get-Command python3).Source
            $moduleName = "robotmk.scheduler"
            $configPath = Join-Path $parentDir "config\robotmk.json"
            $_argumentsList = @("-m", $moduleName, "--config", $configPath)
        }

        return [Command]::new($_executable, $_argumentsList)
    }
}

function StartSchedulerRunner {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory=$true)]
        [string]$configFilePath
    )

    $configFileContent = [Config]::ParseConfigFile($configFilePath)

    $command = [Command]::CreateCommand($configFileContent)
    $parentDir = Split-Path $PSScriptRoot -Parent
    $logPath = Join-Path $parentDir "log\robotmk\scheduler_runner.log"

    # Create processInfo
    $processInfo = New-Object System.Diagnostics.ProcessStartInfo
    $processInfo.FileName = $command.Executable
    $processInfo.Arguments = $command.ArgumentsList
    $processInfo.RedirectStandardOutput = $true
    $processInfo.RedirectStandardError = $true
    $processInfo.UseShellExecute = $false
    $processInfo.CreateNoWindow = $true
    # $processInfo.WorkingDirectory -> Using this we can configure from where the Process will be started
    $processInfo.WorkingDirectory = $PSScriptRoot

    # Create process and assign processInfo to it
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $processInfo


    try {
        $process.Start() | Out-Null

        # Read stdout and stderr and handle them as needed
        while ($true) {
            $stdout = $process.StandardOutput.ReadLine()
            if ($null -ne $stdout) {
                # Handle stdout output here
                WriteLog -Message $stdout -LogPath $logPath
            }

            $stderr = $process.StandardError.ReadLine()
            if ($null -ne $stderr) {
                # Handle stderr output here
                WriteLog -Message $stderr -LogPath $logPath
            }
        }
    }
    catch {
        # TODO: Handle errors
        WriteLog -Message $_.Exception.Message -LogPath $logPath
        throw $_.Exception.Message
    }
}

function WriteLog {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory=$true, Position=0)]
        [string]$Message,

        [Parameter(Mandatory=$true)]
        [string]$LogPath
    )

    # Create the log file and the parent folder if they don't exist
    $LogFolder = Split-Path -Parent $LogPath
    if (-not (Test-Path $LogFolder)) {
        $null = New-Item -Path $LogFolder -ItemType Directory
    }

    if (-not (Test-Path $LogPath)) {
        $null = New-Item -Path $LogPath -ItemType File
    }

    # Get the current timestamp
    $TimeStamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"

    # Format the log entry
    $LogEntry = "$TimeStamp - $Message"

    # Write the log entry to the log file
    $LogEntry | Out-File -Append $LogPath
}
