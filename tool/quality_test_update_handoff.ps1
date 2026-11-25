[CmdletBinding()]
param([string]$CMakePath)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'integration_windows_runner_lifecycle.ps1')

$repository = Get-AmeRepositoryRoot
if (-not $CMakePath) {
    $command = Get-Command cmake -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { $CMakePath = $command.Source }
}
$tools = Get-AmeWindowsRunnerTools -RepositoryRoot $repository -CMakePath $CMakePath
$lock = Enter-AmeRepositoryToolLock
try {
    $output = Join-Path $repository 'build/test/derived/update-handoff'
    Invoke-AmeChecked $tools.CMake @('-S', (Join-Path $repository 'windows/updater'), '-B', $output, '-A', 'x64')
    Invoke-AmeChecked $tools.CMake @('--build', $output, '--config', 'Release', '--target', 'cedarflake_ame_update', 'ame_update_handoff_test')
    Invoke-AmeAcceptanceScratch -Name 'update-handoff' -Action {
        param($scratch)
        Invoke-AmeChecked (Join-Path $output 'Release/ame_update_handoff_test.exe') @($scratch)
    }
} finally { Exit-AmeRepositoryToolLock -Mutex $lock }
