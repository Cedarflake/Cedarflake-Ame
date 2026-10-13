[CmdletBinding()]
param([string]$CMakePath = "")

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_runner_lifecycle.ps1")
. (Join-Path $PSScriptRoot "integration_windows_engine_retirement.ps1")

$toolLock = Enter-AmeRepositoryToolLock
$failure = $null
$unlockFailure = $null
$result = $null
try {
    $repositoryRoot = Get-AmeRepositoryRoot
    $windows = Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot $repositoryRoot -CMakePath $CMakePath
    $engine = Invoke-AmeWindowsEngineRetirement -RepositoryRoot $repositoryRoot -CMakePath $CMakePath
    $result = [pscustomobject]@{ windowLifecycle = $windows; engineRetirement = $engine }
} catch {
    $failure = $_.Exception
} finally {
    try { Exit-AmeRepositoryToolLock $toolLock }
    catch { $unlockFailure = $_.Exception }
}
if ($null -eq $failure) { $failure = $unlockFailure }
if ($null -ne $failure) {
    if ($null -ne $unlockFailure) {
        $failure.Data["ameWindowsRunnerUnlockFailure"] = $unlockFailure.ToString()
    }
    throw $failure
}
$result
