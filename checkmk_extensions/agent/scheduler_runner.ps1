function GetContentFromConfigFile {
    [CmdletBinding()]

    param (
        [Parameter(Mandatory = $true)]
        [string]$configFilePath
    )

    if (-not (Test-Path -Path $configFilePath -PathType Leaf)) {
        # File doesn't exist, so we exit with message
        Write-Error -Message "The file is not available at: $configFilePath"
        Exit 1
    } else {
        # Config file is available
        $configContent = Get-Content -Path $configFilePath
        return $configContent
    }
}

function DetermineRCCValueFromConfig {
    [CmdletBinding()]
    [OutputType([System.Boolean])]

    param (
        [Parameter(Mandatory = $true)]
        [string]$rccValueFromConfigFile
    )

    if ($rccValueFromConfigFile -eq "NO") {
        # The defined value for RCC in the config file is NO
        return $false
    } elseif ($rccValueFromConfigFile -eq "YES") {
        # The defined value for RCC in the config file is YES
        return $true
    } else {
        # Unknown value defined
        Write-Error "The defined value for the RCC is not valid. It must be YES or NO, but it is: $rccValueFromConfigFile" -Category InvalidArgument
        Exit 1
    }
}
function RunRCCOrNot {
    [CmdletBinding()]
    [OutputType([System.Boolean])]

    param (
        [Parameter(Mandatory = $true)]
        [string]$configFilePath
    )

    $configContent = GetContentFromConfigFile $configFilePath
    foreach ($line in $configContent) {
        # Skip empty lines and comments (lines starting with '#')
        if (-not [string]::IsNullOrWhiteSpace($line) -and -not $line.StartsWith("#")) {
            $match = $line | Select-String -Pattern "RCC=(.+)$"

            if (-not $match) {
                # The RCC value is not defined, so we assume it should not run in RCC
                $run_in_rcc = $false
            } else {
                # The RCC value is defined
                $run_in_rcc = DetermineRCCValueFromConfig -rccValueFromConfigFile $match.Matches[0].Groups[1].Value
            }
        }
    }

    return $run_in_rcc
}

