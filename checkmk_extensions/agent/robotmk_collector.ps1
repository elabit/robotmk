Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

function Main {
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigPath
    )
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


function SerializeConfigReadingError {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [System.Management.Automation.ErrorRecord]$Err
    )
    return ConvertTo-Json -InputObject @{ config_reading_error = Out-String -InputObject $Err; }
}


function SerializeConfigFileContent {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigFileContent
    )
    return ConvertTo-Json -InputObject @{ config_file_content = $configFileContent; }
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

Main("${env:MK_CONFDIR}\robotmk.json")
