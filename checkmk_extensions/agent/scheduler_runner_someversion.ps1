class Config {
    [bool]$rcc

    Config() {
        # Default constructor
        # By default, we will not run in RCC
        $this.rcc = $false
    }

    static [System.String] GetConfigContent($configFilePath) {
        try {
            if (-Not (Test-Path -Path $configFilePath -PathType Leaf)) {
                # Throw an error if the file does not exist
                throw "Config file not found: $configFilePath"
            }

            $configFileContent = Get-Content -Raw -Path $configFilePath
            return $configFileContent
        }
        catch {
            # Throw the exception for the parent function to catch it
            throw
        }
    }

    # Method to parse the config file (JSON) and set the value of rcc
    static [Config] ParseConfigFile($configFilePath) {
        $config = [Config]::new()

        try {
            $configFileContent = [Config]::GetConfigContent($configFilePath)

            $configData = ConvertFrom-Json $configFileContent
            if ($configData.rcc -eq "YES") {
                $config.rcc = $true
            }
        }
        catch {
            Write-Host "An error occurred while parsing the JSON: $($_.Exception.Message)" -ForegroundColor Red
            throw # This will ensure that the script exits
        }

        return $config
    }
}

$configPath = "../config/robotm.json"

$configObject = [Config]::ParseConfigFile($configPath)

# Output the value of rcc
if ($configObject.rcc) {
    Write-Host "rcc = Yes" -ForegroundColor Green
} else {
    Write-Host "rcc = No" -ForegroundColor Green
}