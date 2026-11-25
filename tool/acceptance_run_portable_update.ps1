[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$FixtureDirectory,
    [string]$OutputDirectory = 'build/test/derived/application-update-sandbox'
)

. (Join-Path $PSScriptRoot 'acceptance_update_sandbox.ps1')
Invoke-AmeUpdateSandbox -FixtureDirectory $FixtureDirectory -OutputDirectory $OutputDirectory `
    -FixtureFormat 'ame-portable-update-fixture-v1' -ResultFormat 'ame-portable-update-acceptance-v1' `
    -FileNames @('A.zip', 'B.nupkg', 'guest.ps1', 'desktop.ps1', 'user.ps1') -CaseCount 3
