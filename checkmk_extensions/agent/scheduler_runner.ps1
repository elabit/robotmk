# Enable StrictMode 3.0
Set-StrictMode -Version 3.0

class Config {
    [bool]$rcc

    static [string] GetConfigContent($configFilePath) {
        try {
            $configFileContent = Get-Content -Raw -Path $configFilePath -ErrorAction Stop
            return $configFileContent
        }
        catch {
            throw
        }
    }

    # Method to parse the config file (JSON) and set the value of rcc
    static [Config] ParseConfigFile($configFilePath) {
        $config = [Config]::new()

        try {
            $configFileContent = [Config]::GetConfigContent($configFilePath)
            if ($null -ne $configFileContent -and $configFileContent -ne '') {
                $configData = $configFileContent | ConvertFrom-Json -ErrorAction Stop

                # Assign the value from the config file to rcc
                $config.rcc = $configData.PSobject.Properties["rcc"].Value -as [bool]
                }
        }
        catch {
            Write-Host "An error occurred while parsing the config file" -ForegroundColor Red
            throw $_.Exception.Message
        }
        return $config
    }
}
