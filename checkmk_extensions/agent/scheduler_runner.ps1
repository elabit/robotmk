# Enable StrictMode 3.0
Set-StrictMode -Version 3.0

class Config {
    [bool]$rcc

    # Constructor
    # This is a way to make the rcc property mandatory
    Config([bool]$rcc) {
        $this.rcc = $rcc
    }

    # Method to parse the config file (JSON) and set the value of rcc
    static [Config] ParseConfigFile($configFilePath) {

        $configFileContent = Get-Content -Raw -Path $configFilePath -ErrorAction Stop

        try {
            $configData = $configFileContent | ConvertFrom-Json -ErrorAction Stop
            # Assign the value from the config file to rcc
            $use_rcc = $configData.rcc -as [bool]
        }
        catch {
            throw $_.Exception.Message
        }

        return [Config]::new($use_rcc)
    }
}
function CreateCommand([Config]$config) {
    $parentDir = Split-Path $PSScriptRoot -Parent

    if ($config.rcc) {
        $rccPath = Join-Path $PSScriptRoot "rcc.exe"
        $robotPath = Join-Path $parentDir "config\robot.yaml"

        $argumentsList = "run --robot $robotPath"
    }
    else {
        $pythonExe = (Get-Command python).Source
        $moduleName = "robotmk.scheduler"
        $configPath = Join-Path $parentDir "config\robotmk.json"

        $argumentsList = "-m $moduleName --config $configPath"
    }

    return [PSCustomObject]@{
        Executable = if ($config.rcc) { $rccPath } else { $pythonExe }
        Arguments = $argumentsList
    }
}
