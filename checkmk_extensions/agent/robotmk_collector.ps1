Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

function GetResultsDirectory {
    [OutputType([string])]
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$configPath
    )
    $configFileContent = Get-Content -Raw -Path $configPath
    $configData = ConvertFrom-Json -InputObject $configFileContent
    return $configData.results_directory -as [string]
}

$MK_CONFDIR = "C:\robotmk\v2\data\retry_rcc\"

$resultsDirectory = GetResultsDirectory("${MK_CONFDIR}\windows.json")
$files = Get-ChildItem -File -Recurse $resultsDirectory

"<<<robotmk_v2:sep(10)>>>"
foreach ($file in $files) {
	Get-Content -ErrorAction 'SilentlyContinue' $file.FullName
}
