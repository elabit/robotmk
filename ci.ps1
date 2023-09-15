Set-StrictMode -Version 3.0
$ErrorActionPreference = "Stop"

function Main {
    Param (
        [parameter(Mandatory=$true, Position=0)]
        [string]$Mode
    )

    $cargoTomlPath = Join-Path -Path $PSScriptRoot -ChildPath "v2\rust\Cargo.toml"

    switch ( $Mode ) {
        cargo-fmt-check {
            $cargoArgs = @(
                "fmt",
                "--manifest-path",
                $cargoTomlPath,
                "--",
                "--check"
            )
        }
        cargo-clippy {
            $cargoArgs = @(
                "clippy",
                "--manifest-path",
                $cargoTomlPath,
                "--all-targets",
                "--",
                "--deny",
                "warnings"
            )
        }
        cargo-test {
            $cargoArgs = @(
                "test",
                "--manifest-path",
                $cargoTomlPath,
                "--all-targets"
            )
        }
        default {throw "Unknown mode: {0}" -f $Mode}
    }

    $process = Start-Process `
    -FilePath "cargo.exe" `
    -ArgumentList $cargoArgs `
    -NoNewWindow `
    -Wait `
    -PassThru

    if ( $process.ExitCode -eq 0 ) {
        return
    }

    throw "Check failed, see output above"
}

Main($args[0])
