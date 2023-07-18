function Main {
    param (
        [Parameter(Mandatory = $true)]
        [string]$Executable,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [Parameter(Mandatory = $true)]
        [string]$PathPIDFile
    )

    try {
        TerminateProcess -PathPIDFile $PathPIDFile

        while ($true) {
            try {
                $Process = Get-Process -Id (Get-Content -Path $PathPIDFile -ErrorAction Stop) -ErrorAction Stop
            } catch {
                $Process = $null
            }

            if ($null -eq $Process) {
                $Process = StartProcessAndPersistPid -Executable $Executable -Arguments $Arguments -PathPIDFile $PathPIDFile
            }
            else {
                Start-Sleep -Seconds 20
            }
        }
    } catch {
        Write-Host "An error occurred: $_"
        TerminateProcess -PathPIDFile $PathPIDFile
    }
}
function StartProcessAndPersistPid {
    [CmdletBinding()]
    [OutputType([System.Diagnostics.Process])]
    param (
        [Parameter(Mandatory = $true)]
        [string]$Executable,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [Parameter(Mandatory = $true)]
        [string]$PathPIDFile
    )

    $Process = Start-Process -FilePath $Executable -ArgumentList $Arguments -PassThru -ErrorAction Stop
    $Process.Id | Set-Content -Path $PathPIDFile
    return $Process
}

function TerminateProcess {
    # This will stop the process with the PID from the file
    # Also, it will delete the existing file containing the PID
    param (
        [Parameter(Mandatory = $true)]
        [string]$PathPIDFile
    )

    if (Test-Path -Path $PathPIDFile -PathType Leaf) {
        $OtherPid = (Get-Content -Path $PathPIDFile -ErrorAction SilentlyContinue) -as [int]
        if ($OtherPid -and (Get-Process -Id $OtherPid -ErrorAction SilentlyContinue)) {
            Stop-Process -Id $OtherPid -Force -ErrorAction SilentlyContinue
        }
        # Delete file containing PID
        Remove-Item -Path $PathPIDFile -ErrorAction SilentlyContinue
    }
}

try {
    Main -Executable "powershell.exe" -Arguments "-File path/to/powershell_script" -PathPIDFile "current_pid.txt"
} finally {
    TerminateProcess -PathPIDFile "current_pid.txt"
}
