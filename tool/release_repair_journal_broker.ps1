[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "High")]
param(
    [Parameter(Mandatory = $true)]
    [string]$BrokerBinaryPath,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedPublisher
)

$ErrorActionPreference = "Stop"

$arguments = @{
    BrokerBinaryPath = $BrokerBinaryPath
    ExpectedPublisher = $ExpectedPublisher
    Repair = $true
    Confirm = $false
}
if ($WhatIfPreference) {
    $arguments.WhatIf = $true
}
& (Join-Path $PSScriptRoot "release_upgrade_journal_broker.ps1") @arguments
