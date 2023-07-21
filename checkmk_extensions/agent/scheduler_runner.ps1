# Enable StrictMode 3.0
Set-StrictMode -Version 3.0

class Config {
    [bool]$rcc

    static [string] GetConfigContent($configFilePath, [ref]$errorCollection) {
        try {
            $configFileContent = Get-Content -Raw -Path $configFilePath -ErrorAction Stop
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
                $configData = $configFileContent | ConvertFrom-Json

                # Assign the value from the config file to rcc
                $config.rcc = $configData.PSobject.Properties["rcc"].Value -as [bool]
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
