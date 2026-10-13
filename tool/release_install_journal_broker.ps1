[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = "High")]
param(
    [Parameter(Mandatory = $true)]
    [string]$BrokerBinaryPath,
    [string]$ApplicationBundlePath = "",
    [AllowEmptyString()]
    [string]$ExpectedPublisher = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

Assert-AmeBrokerPlatform
Assert-AmeBrokerAdministrator
$sourceBinary = Assert-AmeBrokerPayload `
    -Path $BrokerBinaryPath `
    -ExpectedPublisher $ExpectedPublisher
Assert-AmeBrokerReleaseBinaryName -Path $sourceBinary
$applicationSource = if ($ApplicationBundlePath.Length -gt 0) {
    Assert-AmeApplicationBundleSource `
        -BundlePath $ApplicationBundlePath `
        -ExpectedPublisher $ExpectedPublisher
} else {
    $null
}
$installDirectory = Get-AmeBrokerInstallDirectory
$applicationInstallDirectory = Get-AmeApplicationInstallDirectory
$trustedProgramFilesPath = Get-AmeBrokerProgramFilesX64
Repair-AmeBrokerInterruptedTransaction
Assert-AmeBrokerInstallDirectoryFacts `
    -CandidatePath $installDirectory `
    -TrustedProgramFilesPath $trustedProgramFilesPath
Assert-AmeBrokerFixedInstallTree -InstallDirectory $installDirectory | Out-Null
if ($null -ne $applicationSource) {
    Assert-AmeApplicationFixedInstallTree `
        -InstallDirectory $applicationInstallDirectory | Out-Null
}
$installedBinary = Join-Path $installDirectory (Get-AmeBrokerServicePlan).BinaryName
$installDirectoryAlreadyExisted = Test-Path -LiteralPath $installDirectory -PathType Container

if (Test-AmeBrokerServiceExists) {
    throw "The journal broker service already exists; use repair or upgrade"
}
if (Test-Path -LiteralPath $installedBinary -PathType Leaf) {
    throw "The fixed broker binary already exists without its service"
}
$adoptedEmptyOrphan = $false
if ($installDirectoryAlreadyExisted) {
    Assert-AmeBrokerAdoptableOrphan -InstallDirectory $installDirectory
    $adoptedEmptyOrphan = $true
}
$applicationAlreadyInstalled = Test-Path `
    -LiteralPath $applicationInstallDirectory `
    -PathType Container
if ($null -ne $applicationSource -and $applicationAlreadyInstalled) {
    Assert-AmeApplicationInstallation `
        -InstallDirectory $applicationInstallDirectory `
        -ExpectedPublisher $ExpectedPublisher | Out-Null
}
if (-not $PSCmdlet.ShouldProcess(
    ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($installDirectory))),
    "Install the journal broker service and any explicitly supplied Ame application bundle"
)) {
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
if ($null -ne $applicationSource -and -not $applicationAlreadyInstalled) {
    $applicationTreeFacts = Assert-AmeApplicationFixedInstallTree `
        -InstallDirectory $applicationInstallDirectory
    $plannedOwnedDirectories += @(
        for (
            $index = $applicationTreeFacts.FirstMissingIndex;
            $index -lt $applicationTreeFacts.ExpectedPaths.Count;
            $index += 1
        ) {
            $applicationTreeFacts.ExpectedPaths[$index]
        }
    )
}
$transaction = New-AmeBrokerTransactionState `
    -Operation install `
    -ParentPrestate $parentPrestate `
    -ExpectedPublisher $ExpectedPublisher `
    -OwnedDirectories @(
        @($plannedOwnedDirectories) +
        $(if ($adoptedEmptyOrphan) { $installDirectory }) |
            Select-Object -Unique
    )
try {
    Invoke-AmeBrokerInitialTreeCreation `
        -State $transaction `
        -CreateTreeAction {
            New-AmeBrokerProtectedInstallTree `
                -InstallDirectory $installDirectory | Out-Null
        }
    $stagedBinary = [string]$transaction.StagedBinary
    if (Test-Path -LiteralPath $stagedBinary) {
        throw "The broker install staging path already exists"
    }
    if ($null -ne $applicationSource -and -not $applicationAlreadyInstalled) {
        $transaction.ApplicationOwned = $true
        $transaction.Phase = "installing_application"
        Write-AmeBrokerTransactionState -State $transaction
        $applicationState = Install-AmeApplicationBundle `
            -SourceBundlePath $applicationSource.BundlePath `
            -InstallDirectory $applicationInstallDirectory `
            -ExpectedPublisher $ExpectedPublisher
        $transaction.OwnedDirectories = @(
            @($transaction.OwnedDirectories) + @($applicationState.CreatedDirectories) |
                Select-Object -Unique
        )
        Write-AmeBrokerTransactionState -State $transaction
    }
    Copy-AmeBrokerStagedPayload `
        -SourcePath $sourceBinary `
        -StagedPath $stagedBinary `
        -IncludeServiceSid $false `
        -ExpectedPublisher $ExpectedPublisher
    $transaction.Phase = "staged_binary"
    Write-AmeBrokerTransactionState -State $transaction
    $identityManifest = New-AmeBrokerIdentityManifest `
        -BinaryPath $stagedBinary `
        -ExpectedPublisher $ExpectedPublisher
    $transaction.ExpectedNewBinary = Get-AmeBrokerTransactionBinaryFacts `
        -Path $stagedBinary `
        -ExpectedPublisher $ExpectedPublisher
    $transaction.NewIdentityManifest = $identityManifest
    Write-AmeBrokerTransactionState -State $transaction
    Invoke-AmeBrokerWriteAheadMove `
        -State $transaction `
        -SourcePath $stagedBinary `
        -DestinationPath $installedBinary `
        -ExpectedSource $transaction.ExpectedNewBinary `
        -ExpectedDestinationBefore (New-AmeBrokerAbsentBinaryFacts) `
        -NextPhase installed_binary
    $transaction.InstalledOwned = $true
    Write-AmeBrokerTransactionState -State $transaction
    $serviceCreated = $false
    Set-AmeBrokerServiceConfiguration `
        -BinaryPath $installedBinary `
        -IdentityManifest $identityManifest `
        -Create `
        -CreatedState ([ref]$serviceCreated)
    $transaction.ServiceCreated = $serviceCreated
    $transaction.Phase = "service_created"
    Write-AmeBrokerTransactionState -State $transaction
    Set-AmeBrokerBinaryAcl `
        -InstallDirectory $installDirectory `
        -BinaryPath $installedBinary
    Assert-AmeBrokerInstallation `
        -BinaryPath $installedBinary `
        -ExpectedPublisher $ExpectedPublisher
    $transaction.Phase = "committed"
    Write-AmeBrokerTransactionState -State $transaction
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

Write-Output "journal_broker_installed"
