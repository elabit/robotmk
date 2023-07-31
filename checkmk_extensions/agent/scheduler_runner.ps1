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
    if ($config.rcc) {
        if (-Not (Test-Path -Path "./rcc.exe" -PathType Leaf)) {
           $errorMessage = "Error: 'rcc.exe' binary not found in the current folder."
            throw New-Object System.IO.FileNotFoundException -ArgumentList $errorMessage
        }
        # TODO: Add the arguments.
        # What do I need?
        # Simon is calculating the blueprint here
        return ("./rcc.exe")
    }
    else {
        $pythonExe = (Get-Command python).Source # Should we use python or python3 here?
        return ($pythonExe)
    }
}
