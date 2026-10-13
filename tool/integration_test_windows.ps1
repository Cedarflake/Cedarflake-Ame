[CmdletBinding()]
param(
    [ValidateRange(60, 1800)]
    [int]$TimeoutSeconds = 900
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_scan_run.ps1")

$toolchain = Get-AmeToolchain
$outcome = Invoke-AmeWindowsScanRun `
    -RepositoryRoot (Get-AmeRepositoryRoot) `
    -FlutterPath $toolchain.Flutter `
    -TimeoutSeconds $TimeoutSeconds
Complete-AmeWindowsScanRun -Outcome $outcome
