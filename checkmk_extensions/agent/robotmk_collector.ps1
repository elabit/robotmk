Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

function Main {
    $ConfigPath = DetermineConfigPath -CommandLineArgs $args
    Write-Output "<<<robotmk_v2:sep(10)>>>"

    try {
        $configFileContent = Get-Content -Raw -Path $ConfigPath
    }
    catch {
        Write-Output (SerializeConfigReadingError($Error[0]))
        Throw $Error[0]
    }

    # We don't know if the config is actually valid, which is why don't simply dump it as is but
    # instead wrap it.
    Write-Output (SerializeConfigFileContent($configFileContent))

    $files = Get-ChildItem -File -Recurse (GetResultsDirectory($configFileContent))
    foreach ($file in $files) {
        Write-Output (Get-Content -Raw -ErrorAction 'SilentlyContinue' $file.FullName)
    }
}


function DetermineConfigPath {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$false, Position=0)]
        [String[]]$CommandLineArgs
    )

    if ($CommandLineArgs -ne "" -and $CommandLineArgs.Count -gt 0) {
        return $CommandLineArgs[0]
    }

    $configDir = $env:MK_CONFDIR
    if ($null -eq $configDir) {
        $configDir = 'C:\ProgramData\checkmk\agent\config'
    }

    return Join-Path $configDir 'robotmk.json'
}
function SerializeConfigReadingError {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [System.Management.Automation.ErrorRecord]$Err
    )
    return ConvertTo-Json -Compress -InputObject @{ config_reading_error = Out-String -InputObject $Err; }
}


function SerializeConfigFileContent {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigFileContent
    )
    return ConvertTo-Json -Compress -InputObject @{ config_file_content = $configFileContent; }
}

function GetResultsDirectory {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigFileContent
    )
    $configData = ConvertFrom-Json -InputObject $configFileContent
    return $configData.results_directory -as [string]
}

Main $args
