[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "High")]
param(
    [Parameter(Mandatory = $true)]
    [string]$BrokerBinaryPath,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedPublisher,
    [switch]$Repair
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

Assert-AmeBrokerPlatform
Assert-AmeBrokerAdministrator
$sourceBinary = Assert-AmeBrokerBinary `
    -Path $BrokerBinaryPath `
    -ExpectedPublisher $ExpectedPublisher
Assert-AmeBrokerReleaseBinaryName -Path $sourceBinary
$applicationInstallDirectory = Get-AmeApplicationInstallDirectory
$installedClientBinary = Assert-AmeApplicationInstallation `
    -InstallDirectory $applicationInstallDirectory `
    -ExpectedPublisher $ExpectedPublisher
$newIdentityManifest = New-AmeBrokerIdentityManifest `
    -BinaryPath $sourceBinary `
    -ClientBinaryPath $installedClientBinary `
    -ExpectedPublisher $ExpectedPublisher
$installDirectory = Get-AmeBrokerInstallDirectory
$trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
Repair-AmeBrokerInterruptedTransaction
Assert-AmeBrokerInstallDirectoryFacts `
    -CandidatePath $installDirectory `
    -TrustedProgramFilesPath $trustedProgramFilesPath
Assert-AmeBrokerFixedInstallTree `
    -InstallDirectory $installDirectory `
    -RequireInstallDirectory | Out-Null
$installedBinary = Join-Path $installDirectory (Get-AmeBrokerServicePlan).BinaryName
$installedBinaryExists = Test-Path -LiteralPath $installedBinary -PathType Leaf

if (-not (Test-AmeBrokerServiceExists)) {
    throw "The journal broker service is not installed"
}
$previousIdentityManifest = Get-AmeBrokerConfiguredIdentityManifest
Assert-AmeBrokerServiceConfiguration `
    -BinaryPath $installedBinary `
    -IdentityManifest $previousIdentityManifest
if (-not $Repair -and -not $installedBinaryExists) {
    throw "The installed journal broker binary is missing; use repair"
}
Assert-AmeBrokerExactAclFacts `
    -Facts (Get-AmeBrokerAclFacts -Path $installDirectory) `
    -Kind "Directory" `
    -IncludeServiceSid $true
if ($installedBinaryExists) {
    Assert-AmeBrokerPhysicalFile -ExpectedPath $installedBinary
    Assert-AmeBrokerExactAclFacts `
        -Facts (Get-AmeBrokerAclFacts -Path $installedBinary) `
        -Kind "File" `
        -IncludeServiceSid $true
    Assert-AmeBrokerBinary `
        -Path $installedBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    $installedIdentityManifest = New-AmeBrokerIdentityManifest `
        -BinaryPath $installedBinary `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher
    if ($installedIdentityManifest -cne $previousIdentityManifest) {
        throw "The installed broker identity does not match the protected service manifest"
    }
}
$service = Get-Service -Name (Get-AmeBrokerServicePlan).ServiceName
try {
    $wasRunning = $service.Status -ne `
        [System.ServiceProcess.ServiceControllerStatus]::Stopped
} finally {
    $service.Dispose()
}
if (-not $installedBinaryExists -and $wasRunning) {
    throw "A running service with a missing binary cannot be repaired transactionally"
}
if (-not $PSCmdlet.ShouldProcess($installedBinary, "Atomically upgrade the journal broker")) {
    return
}

$parentPrestate = Get-AmeBrokerParentPrestate
$operation = if ($Repair) { "repair" } else { "upgrade" }
$transaction = New-AmeBrokerTransactionState `
    -Operation $operation `
    -ParentPrestate $parentPrestate `
    -ExpectedPublisher $ExpectedPublisher
$transaction.PreviousIdentityManifest = $previousIdentityManifest
$transaction.WasRunning = $wasRunning
$transaction.ExpectedPreviousBinary = if ($installedBinaryExists) {
    Get-AmeBrokerTransactionBinaryFacts `
        -Path $installedBinary `
        -ExpectedPublisher $ExpectedPublisher
} else {
    New-AmeBrokerAbsentBinaryFacts
}
$stagedBinary = [string]$transaction.StagedBinary
$backupBinary = [string]$transaction.BackupBinary
if ((Test-Path -LiteralPath $stagedBinary) -or (Test-Path -LiteralPath $backupBinary)) {
    throw "A broker upgrade transaction path already exists"
}
Write-AmeBrokerTransactionState -State $transaction
try {
    Stop-AmeBrokerServiceBounded
    Copy-Item -LiteralPath $sourceBinary -Destination $stagedBinary
    $transaction.Phase = "staged_binary"
    Write-AmeBrokerTransactionState -State $transaction
    Set-AmeBrokerFileAcl -Path $stagedBinary -IncludeServiceSid $true
    Assert-AmeBrokerBinary `
        -Path $stagedBinary `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
    $transaction.ExpectedNewBinary = Get-AmeBrokerTransactionBinaryFacts `
        -Path $stagedBinary `
        -ExpectedPublisher $ExpectedPublisher
    $transaction.NewIdentityManifest = $newIdentityManifest
    Write-AmeBrokerTransactionState -State $transaction
    if ($installedBinaryExists) {
        $transaction.ConfigurationChanged = $true
        $transaction.Phase = "service_holds_staged_pending"
        Write-AmeBrokerTransactionState -State $transaction
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath $stagedBinary `
            -IdentityManifest $newIdentityManifest
        Invoke-AmeBrokerWriteAheadMove `
            -State $transaction `
            -SourcePath $installedBinary `
            -DestinationPath $backupBinary `
            -ExpectedSource $transaction.ExpectedPreviousBinary `
            -ExpectedDestinationBefore (New-AmeBrokerAbsentBinaryFacts) `
            -NextPhase backup_created
        $transaction.BackupCreated = $true
        Write-AmeBrokerTransactionState -State $transaction
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath $backupBinary `
            -IdentityManifest $previousIdentityManifest
    } else {
        Copy-Item -LiteralPath $stagedBinary -Destination $backupBinary
        Set-AmeBrokerFileAcl -Path $backupBinary -IncludeServiceSid $true
        Assert-AmeBrokerBinary `
            -Path $backupBinary `
            -ExpectedPublisher $ExpectedPublisher | Out-Null
        $transaction.BackupCreated = $true
        $transaction.ConfigurationChanged = $true
        $transaction.Phase = "repair_holding_binary"
        Write-AmeBrokerTransactionState -State $transaction
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath $backupBinary `
            -IdentityManifest $newIdentityManifest
    }
    Invoke-AmeBrokerWriteAheadMove `
        -State $transaction `
        -SourcePath $stagedBinary `
        -DestinationPath $installedBinary `
        -ExpectedSource $transaction.ExpectedNewBinary `
        -ExpectedDestinationBefore (New-AmeBrokerAbsentBinaryFacts) `
        -NextPhase installed_binary
    $transaction.InstalledOwned = $true
    Write-AmeBrokerTransactionState -State $transaction
    $transaction.Phase = "changing_service_configuration"
    Write-AmeBrokerTransactionState -State $transaction
    Set-AmeBrokerServiceConfiguration `
        -BinaryPath $installedBinary `
        -IdentityManifest $newIdentityManifest
    Set-AmeBrokerBinaryAcl `
        -InstallDirectory $installDirectory `
        -BinaryPath $installedBinary
    Assert-AmeBrokerInstallation `
        -BinaryPath $installedBinary `
        -ClientBinaryPath $installedClientBinary `
        -ExpectedPublisher $ExpectedPublisher
    if ($wasRunning) {
        Start-AmeBrokerServiceBounded
    }
    $transaction.Phase = "committed"
    Write-AmeBrokerTransactionState -State $transaction
    $ownership = [pscustomobject]@{ BackupCreated = [bool]$transaction.BackupCreated }
    Remove-AmeBrokerTransactionBackup `
        -Ownership $ownership `
        -BackupPath $backupBinary
    $transaction.BackupCreated = [bool]$ownership.BackupCreated
    Remove-AmeBrokerTransactionMarker
} catch {
    $originalError = $_
    $rollbackFailures = @()
    try {
        Repair-AmeBrokerInterruptedTransaction -ProcessProbe {
            param([int]$ProcessId)
            [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
        }
    } catch {
        $rollbackFailures += $_.Exception.Message
    }
    Throw-AmeBrokerTransactionFailure `
        -OriginalError $originalError `
        -RollbackFailures $rollbackFailures
}

Write-Output "journal_broker_upgraded"
