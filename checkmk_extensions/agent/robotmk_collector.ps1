Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

function Main {
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigPath
    )

    $resultsDirectory = GetResultsDirectory($ConfigPath)
    $files = Get-ChildItem -File -Recurse $resultsDirectory

    Write-Output "<<<robotmk_v2:sep(10)>>>"
    foreach ($file in $files) {
        Write-Output (Get-Content -Raw -ErrorAction 'SilentlyContinue' $file.FullName)
    }
}

function GetResultsDirectory {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$ConfigPath
    )
    $configFileContent = Get-Content -Raw -Path $ConfigPath
    $configData = ConvertFrom-Json -InputObject $configFileContent
    return $configData.results_directory -as [string]
}

Main("${env:MK_CONFDIR}\robotmk.json")
