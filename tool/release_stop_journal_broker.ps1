[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "High")]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

Assert-AmeBrokerPlatform
Assert-AmeBrokerAdministrator
if (-not (Test-AmeBrokerServiceExists)) {
    Write-Output "journal_broker_absent"
    return
}
if ($PSCmdlet.ShouldProcess(
    (Get-AmeBrokerServicePlan).ServiceName,
    "Stop the journal broker service"
)) {
    Stop-AmeBrokerServiceBounded
}
Write-Output "journal_broker_stopped"
