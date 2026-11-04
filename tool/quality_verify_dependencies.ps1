$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock
Push-Location (Get-AmeRepositoryRoot)
try {
    Invoke-AmeChecked $toolchain.Dart @("run", "tool/dependency-boundaries/verify.dart")
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
