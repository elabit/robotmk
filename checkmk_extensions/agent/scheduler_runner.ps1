# Enable StrictMode 3.0
Set-StrictMode -Version 3.0

class Config {
    [bool]$rcc

    Config() {
        # Default constructor
        # By default, we will not run in RCC
        $this.rcc = $false
    }

    static [string] GetConfigContent($configFilePath, [ref]$errorCollection) {
        try {
            if (-Not (Test-Path -Path $configFilePath -PathType Leaf)) {
                # Add the error message to the error collection
                $errorCollection.Value.Add("Config file not found: $configFilePath")

                # If we want to throw an ERRORRECORD object or a .NET exception, we can use the something like:
                # $errorCollection.Value.Add((New-Object System.IO.FileNotFoundException))
                return $null
            }

            $configFileContent = Get-Content -Raw -Path $configFilePath
            return $configFileContent
        }
        catch {
            # Add the error message to the error collection
            $errorCollection.Value.Add($_.Exception.Message)
            return $null
        }
    }

    # Method to parse the config file (JSON) and set the value of rcc
    static [Config] ParseConfigFile($configFilePath) {
        $config = [Config]::new()
        $errors = [System.Collections.Generic.List[string]]::new()

        try {
            $configFileContent = [Config]::GetConfigContent($configFilePath, ([ref]$errors))
            if ($null -ne $configFileContent -and $configFileContent -ne '') {
                $configData = ConvertFrom-Json $configFileContent
                # Check if it contains the rcc propetry and if it's equal to YES
                if ($configData.PSobject.Properties["rcc"] -and $configData.rcc -eq "YES") {
                    $config.rcc = $true
                    }
                }
        }
        catch {
            Write-Host "An error occurred while parsing the config file" -ForegroundColor Red
            # Add the error message to the error collection
            $errors.Add($_.Exception.Message)
        }

        # Check if there are any errors and throw them
        if ($errors.Count -gt 0) {
            $errorMessages = $errors -join "`n"
            throw $errorMessages
        }

        return $config
    }
}
