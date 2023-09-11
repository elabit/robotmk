Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

class RCCConfig {
    [string]$BinaryPath
    [string]$SchedulerRobotYamlPath

    RCCConfig([string]$BinaryPath, [string]$SchedulerRobotYamlPath) {
        $this.BinaryPath = $BinaryPath
        $this.SchedulerRobotYamlPath = $SchedulerRobotYamlPath
    }

    static [RCCConfig] ParseRawConfig([object]$RawConfig) {
        return [RCCConfig]::new(
            $RawConfig.rcc_binary_path -as [string],
            $RawConfig.scheduler_robot_yaml_path -as [string]
        )
    }
}

class Config {
    [AllowNull()] [RCCConfig]$RCCConfig
    [string]$ResultsDirectory
    [string]$LogDirectory

    Config(
        [string]$ResultsDirectory,
        [string]$LogDirectory
    ) {
        $this.RCCConfig = $null
        $this.ResultsDirectory = $ResultsDirectory
        $this.LogDirectory = $LogDirectory
    }

    Config(
        [RCCConfig]$RCCConfig,
        [string]$ResultsDirectory,
        [string]$LogDirectory
    ) {
        $this.RCCConfig = $RCCConfig
        $this.ResultsDirectory = $ResultsDirectory
        $this.LogDirectory = $LogDirectory
    }

    static [Config] ParseConfigFile([string]$Path) {
        $configFileContent = Get-Content -Raw -Path $Path
        $configData = ConvertFrom-Json -InputObject $configFileContent

        $resultsDir = $configData.results_directory -as [string]
        $logDir = $configData.log_directory -as [string]

        if($configData.environment -eq "system_python") {
            return [Config]::new(
                $resultsDir,
                $logDir
                )
        }
        return [Config]::new(
            [RCCConfig]::ParseRawConfig($configData.environment),
            $resultsDir,
            $logDir
        )
    }
}

class CommandSpecification {
    [string]$Executable
    [string[]]$Arguments

    CommandSpecification([string]$Executable, [string[]]$Arguments) {
        $this.Executable = $Executable
        $this.Arguments = $Arguments
    }
}

function CreateSchedulerExecCommand {
    [CmdletBinding()]
    [OutputType([CommandSpecification])]
    param (
        [Parameter(Mandatory=$true, Position=0)]
        [string]$ConfigPath,

        [Parameter(Mandatory=$true, Position=1)]
        [AllowNull()]
        [RCCConfig]$RCCConfig
    )
    $pythonSchedArgs = @("-m", "robotmk", $ConfigPath)

    if ($null -eq $RCCConfig) {
        return [CommandSpecification]::new(
            (Get-Command python).Source,
            $pythonSchedArgs
        )
    }
    return [CommandSpecification]::new(
        $RCCConfig.BinaryPath,
        @(
            "task",
            "script",
            "--controller",
            "robotmk",
            "--space",
            "scheduler",
            "--robot",
            $RCCConfig.SchedulerRobotYamlPath,
            "--",
            "python"
        ) + $pythonSchedArgs
    )
}

function StartSchedulerRunner {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory=$true)]
        [string]$ConfigFilePath
    )

    $config = [Config]::ParseConfigFile($ConfigFilePath)
    $commandSpec = CreateSchedulerExecCommand $ConfigFilePath $config.RCCConfig

    if (-not (Test-Path $config.LogDirectory)) {
        New-Item -Path $config.LogDirectory -ItemType Directory
    }

    $selfLogPath = Join-Path $config.LogDirectory "scheduler_runner.log"
    $schedulerStdoutLogPath = Join-Path $config.LogDirectory "scheduler_stdout.log"
    $schedulerStderrLogPath = Join-Path $config.LogDirectory "scheduler_stderr.log"

    while ($true) {
        try {
            Start-Process `
            -FilePath $commandSpec.Executable `
            -ArgumentList $commandSpec.Arguments `
            -Wait `
            -NoNewWindow `
            -RedirectStandardOutput $SchedulerStdoutLogPath `
            -RedirectStandardError $SchedulerStderrLogPath
        }
        catch {
            # TODO: Handle errors
            WriteLog -Message $_.Exception.Message -LogPath $selfLogPath
        }
    }
}

function WriteLog {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory=$true, Position=0)]
        [string]$Message,

        [Parameter(Mandatory=$true, Position=1)]
        [string]$LogPath
    )

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

StartSchedulerRunner $args[0]
