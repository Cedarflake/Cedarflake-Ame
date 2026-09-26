$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "acceptance_r2c_change_driven_reliability_common.ps1")

$validContext = @{
    IsWindowsPlatform = $true
    OperatingSystemArchitecture = [Runtime.InteropServices.Architecture]::X64
    ProcessArchitecture = [Runtime.InteropServices.Architecture]::X64
    BuildNumber = 22621
    ApiBuildNumber = 22621
    InstallationType = "Client"
    ProductType = "WinNT"
    ProductSku = [uint32]48
    IsAdministrator = $false
}
Assert-AmeR2cRExecutionContext @validContext

$nonWindowsContext = @{} + $validContext
$nonWindowsContext.IsWindowsPlatform = $false
$rejected = $false
try {
    Assert-AmeR2cRExecutionContext @nonWindowsContext
} catch {
    if ($_.Exception.Message -cne "R2c-R change-driven reliability requires Windows") {
        throw "The R2c-R platform probe returned an unexpected diagnostic"
    }
    $rejected = $true
}
if (-not $rejected) {
    throw "The R2c-R platform probe accepted a non-Windows platform"
}

Write-Output "windows_powershell_platform_compatibility_passed"
