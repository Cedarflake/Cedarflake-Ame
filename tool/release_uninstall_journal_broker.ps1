[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "High")]
param(
    [AllowEmptyString()]
    [string]$ExpectedPublisher = "",
    [switch]$RemoveApplicationBundle
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

Assert-AmeBrokerPlatform
Assert-AmeBrokerAdministrator
$installDirectory = Get-AmeBrokerInstallDirectory
$trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
Repair-AmeBrokerInterruptedTransaction
Assert-AmeBrokerInstallDirectoryFacts `
    -CandidatePath $installDirectory `
    -TrustedProgramFilesPath $trustedProgramFilesPath
$installedBinary = Join-Path $installDirectory (Get-AmeBrokerServicePlan).BinaryName
$applicationInstallDirectory = Get-AmeApplicationInstallDirectory
if (Test-Path -LiteralPath $installDirectory -PathType Container) {
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory $installDirectory `
        -RequireInstallDirectory | Out-Null
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $installDirectory) `
        -Kind "Directory" `
        -IncludeServiceSid $true
}
if (Test-Path -LiteralPath $installedBinary -PathType Leaf) {
    Assert-AmeBrokerPhysicalFile -ExpectedPath $installedBinary
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $installedBinary) `
        -Kind "File" `
        -IncludeServiceSid $true
    Assert-AmeBrokerPayload `
        -Path $installedBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
}
if ($RemoveApplicationBundle -and
    (Test-Path -LiteralPath $applicationInstallDirectory -PathType Container)) {
    Assert-AmeApplicationInstallation `
        -InstallDirectory $applicationInstallDirectory `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
}
if (Test-AmeBrokerServiceExists) {
    $configuredIdentityManifest = Get-AmeBrokerConfiguredIdentityManifest
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $installedBinary `
        -IdentityManifest $configuredIdentityManifest
    if (Test-Path -LiteralPath $installedBinary -PathType Leaf) {
        Assert-AmeBrokerManifestBinary `
            -BinaryPath $installedBinary `
            -Manifest $configuredIdentityManifest
    }
}
$removalDescription = if ($RemoveApplicationBundle) {
    "Remove the installed Ame Application bundle and journal broker"
} else {
    "Remove the installed journal broker"
}
if (-not $PSCmdlet.ShouldProcess($installDirectory, $removalDescription)) {
    return
}

$parentPrestate = Get-AmeBrokerParentPrestate
$brokerTreeFacts = Assert-AmeBrokerFixedInstallTree -InstallDirectory $installDirectory
$plannedOwnedDirectories = @(
    for (
        $index = $brokerTreeFacts.FirstMissingIndex;
        $index -lt $brokerTreeFacts.ExpectedPaths.Count;
        $index += 1
    ) {
        $brokerTreeFacts.ExpectedPaths[$index]
    }
)
$transaction = $null
try {
    $transaction = New-AmeBrokerTransactionState `
        -Operation uninstall `
        -ParentPrestate $parentPrestate `
        -ExpectedPublisher $ExpectedPublisher `
        -OwnedDirectories $plannedOwnedDirectories
    $transaction.RemoveApplicationBundle = [bool]$RemoveApplicationBundle
    Write-AmeBrokerTransactionState -State $transaction
    Complete-AmeBrokerUninstallTransaction -State $transaction
} catch {
    $originalError = $_
    $recoveryFailures = @()
    if ($null -ne $transaction) {
        try {
            Repair-AmeBrokerInterruptedTransaction -ProcessProbe {
                param([int]$ProcessId)
                [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
            }
        } catch {
            $recoveryFailures += $_.Exception.Message
        }
    }
    Throw-AmeBrokerTransactionFailure `
        -OriginalError $originalError `
        -RollbackFailures $recoveryFailures
}

Write-Output "journal_broker_uninstalled"
