[CmdletBinding()]
param(
    [switch]$Check
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$dartPaths = @("lib", "test", "integration_test")
$toolLock = Enter-AmeRepositoryToolLock

Push-Location $repositoryRoot
try {
    $rustArguments = @("fmt", "--manifest-path", "rust\Cargo.toml", "--")
    if ($Check) {
        $rustArguments += "--check"
    }
    Invoke-AmeChecked $toolchain.Cargo $rustArguments

    $dartArguments = @("format")
    if ($Check) {
        $dartArguments += @("--output=none", "--set-exit-if-changed")
    }
    $dartArguments += $dartPaths
    Invoke-AmeChecked $toolchain.Dart $dartArguments
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
