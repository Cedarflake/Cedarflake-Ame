[CmdletBinding()]
param(
    [Parameter(Mandatory, ParameterSetName = 'Start')][string]$FixtureDirectory,
    [string]$OutputDirectory = 'build/test/derived/application-update-installed-sandbox',
    [Parameter(Mandatory, ParameterSetName = 'Resume')][switch]$Resume,
    [Parameter(ParameterSetName = 'Resume')][switch]$UiVerified,
    [Parameter(ParameterSetName = 'Resume')][switch]$Retire
)

. (Join-Path $PSScriptRoot 'acceptance_update_sandbox_owner.ps1')
if ($Resume) {
    Resume-AmeInstalledUpdateSandbox -OutputDirectory $OutputDirectory -UiVerified:$UiVerified -Retire:$Retire
    return
}
. (Join-Path $PSScriptRoot 'acceptance_update_sandbox.ps1')
$report = Invoke-AmeUpdateSandbox -FixtureDirectory $FixtureDirectory -OutputDirectory $OutputDirectory `
    -FixtureFormat 'ame-installed-update-fixture-v1' -ResultFormat 'ame-installed-update-acceptance-v1' `
    -FileNames @('baseline.msi', 'target.msi', 'vc_redist.exe', 'parent.exe', 'helper.exe',
        'volume.ps1', 'guest.ps1', 'worker.ps1', 'user.ps1', 'desktop.ps1') -CaseCount 1 -TimeoutMinutes 15 -AllowUserHandoff
$result = Get-Content -LiteralPath $report -Raw -Encoding UTF8 | ConvertFrom-Json
if ($result.format -ceq 'ame-owned-sandbox-v1' -and $result.state -ceq 'awaiting-user') {
    Write-Output $report
    return
}
if ($result.uninstalled -ne $true) { throw 'Updated product did not uninstall from the disposable guest' }
Write-Output $report
