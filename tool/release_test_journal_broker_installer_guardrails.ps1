$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")
$repositoryRoot = Split-Path -Parent $PSScriptRoot

function Assert-Rejected {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Action,
        [Parameter(Mandatory = $true)]
        [string]$Message,
        [AllowNull()]
        [string]$Expected = $null
    )

    $rejected = $false
    try {
        & $Action
    } catch {
        if ($null -ne $Expected -and
            $_.Exception.Message.IndexOf($Expected, [StringComparison]::Ordinal) -lt 0) {
            throw "Expected broker guardrail rejection to contain: $Expected"
        }
        $rejected = $true
    }
    if (-not $rejected) {
        throw $Message
    }
}

Assert-AmeBrokerPlatformFacts `
    -IsWindowsPlatform $true `
    -Is64BitOperatingSystem $true `
    -Is64BitProcess $true `
    -BuildNumber 22621 `
    -ProductType 1
Assert-Rejected `
    -Action {
        Assert-AmeBrokerPlatformFacts `
            -IsWindowsPlatform $false `
            -Is64BitOperatingSystem $true `
            -Is64BitProcess $true `
            -BuildNumber 22621 `
            -ProductType 1
    } `
    -Message "The broker platform guard accepted a non-Windows platform" `
    -Expected "The journal broker supports only Windows 11 x64"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerPlatformFacts `
            -IsWindowsPlatform $true `
            -Is64BitOperatingSystem $false `
            -Is64BitProcess $true `
            -BuildNumber 22621 `
            -ProductType 1
    } `
    -Message "The broker platform guard accepted a 32-bit operating system"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerPlatformFacts `
            -IsWindowsPlatform $true `
            -Is64BitOperatingSystem $true `
            -Is64BitProcess $false `
            -BuildNumber 22621 `
            -ProductType 1
    } `
    -Message "The broker platform guard accepted a WOW64 installer process"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerPlatformFacts `
            -IsWindowsPlatform $true `
            -Is64BitOperatingSystem $true `
            -Is64BitProcess $true `
            -BuildNumber 19045 `
            -ProductType 1
    } `
    -Message "The broker platform guard accepted Windows 10"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerPlatformFacts `
            -IsWindowsPlatform $true `
            -Is64BitOperatingSystem $true `
            -Is64BitProcess $true `
            -BuildNumber 26100 `
            -ProductType 3
    } `
    -Message "The broker platform guard accepted a Windows server SKU"

$publisher = "CN=Cedarflake Ame Release Test"
Assert-AmeBrokerSignatureFacts `
    -Status "Valid" `
    -SignerSubject $publisher `
    -ExpectedPublisher $publisher
Assert-Rejected `
    -Action {
        Assert-AmeBrokerSignatureFacts `
            -Status "NotSigned" `
            -SignerSubject $null `
            -ExpectedPublisher $publisher
    } `
    -Message "The broker signature guard accepted an unsigned binary"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerSignatureFacts `
            -Status "Valid" `
            -SignerSubject "CN=Unexpected Publisher" `
            -ExpectedPublisher $publisher
    } `
    -Message "The broker signature guard accepted another valid publisher"

$identityDigest = "05" * 32
$identityPublisher = ConvertTo-AmeBrokerUpperHex `
    -Bytes ([Text.Encoding]::Unicode.GetBytes($publisher))
$identityClientPath = ConvertTo-AmeBrokerUpperHex `
    -Bytes ([Text.Encoding]::Unicode.GetBytes(
        (Get-AmeApplicationInstalledBinaryPath)
    ))
$identityManifest = "AMEJBID2|protocol=5|max_frame=1048576|sha256=$identityDigest|" +
    "signer_sha256=$identityDigest|publisher_utf16le=$identityPublisher|" +
    "client_path_utf16le=$identityClientPath|client_sha256=$identityDigest|" +
    "client_signer_sha256=$identityDigest"
Assert-AmeBrokerIdentityManifestFacts -Manifest $identityManifest
foreach ($invalidManifest in @(
    $identityManifest.Replace("AMEJBID2", "AMEJBID1"),
    $identityManifest.Replace("protocol=5", "protocol=4"),
    $identityManifest.Replace("max_frame=1048576", "max_frame=0"),
    $identityManifest.Replace("sha256=$identityDigest", "sha256=05"),
    $identityManifest.Replace("client_path_utf16le=$identityClientPath", "client_path_utf16le=4300"),
    ($identityManifest + "|extra=1")
)) {
    Assert-Rejected `
        -Action { Assert-AmeBrokerIdentityManifestFacts -Manifest $invalidManifest } `
        -Message "The broker identity manifest guard accepted malformed identity evidence"
}

Assert-AmeBrokerMachineFacts -Machine 0x8664
Assert-Rejected `
    -Action { Assert-AmeBrokerMachineFacts -Machine 0x014C } `
    -Message "The broker binary guard accepted an x86 PE image"
Assert-Rejected `
    -Action { Assert-AmeBrokerMachineFacts -Machine 0xAA64 } `
    -Message "The broker binary guard accepted an ARM64 PE image"

$protocolFacts = Assert-AmeBrokerProtocolOutputFacts `
    -Output @("binary=cedarflake_ame_journal_broker protocol=5 max_frame=1048576") `
    -ExitCode 0
if ($protocolFacts.Protocol -ne 5 -or $protocolFacts.MaximumFrameBytes -ne 1048576) {
    throw "The broker protocol guard did not retain the exact compatible facts"
}
foreach ($invalidProtocolProbe in @(
    [pscustomobject]@{
        Output = @("binary=cedarflake_ame_journal_broker protocol=4 max_frame=1048576")
        ExitCode = 0
    },
    [pscustomobject]@{
        Output = @("binary=cedarflake_ame_journal_broker protocol=5 max_frame=1048576", "extra")
        ExitCode = 0
    },
    [pscustomobject]@{
        Output = @("binary=cedarflake_ame_journal_broker protocol=5 max_frame=1048576")
        ExitCode = 1
    },
    [pscustomobject]@{
        Output = @("binary=other protocol=5 max_frame=1048576")
        ExitCode = 0
    }
)) {
    Assert-Rejected `
        -Action {
            Assert-AmeBrokerProtocolOutputFacts `
                -Output $invalidProtocolProbe.Output `
                -ExitCode $invalidProtocolProbe.ExitCode
        } `
        -Message "The broker protocol guard accepted incompatible executable output"
}

$rollbackTrace = [System.Collections.Generic.List[string]]::new()
$rollbackActions = @(
    New-AmeBrokerRollbackAction -Name "first" -Action {
        $rollbackTrace.Add("first")
    }
    New-AmeBrokerRollbackAction -Name "second" -Action {
        $rollbackTrace.Add("second")
        throw "injected rollback failure"
    }
    New-AmeBrokerRollbackAction -Name "third" -Action {
        $rollbackTrace.Add("third")
    }
)
$rollbackFailures = @(Invoke-AmeBrokerRollbackActions -Actions $rollbackActions)
if (($rollbackTrace -join ",") -cne "third,second,first") {
    throw "The broker rollback journal did not execute every action in reverse order"
}
if (
    $rollbackFailures.Count -ne 1 -or
    -not $rollbackFailures[0].StartsWith("second:", [StringComparison]::Ordinal)
) {
    throw "The broker rollback journal did not isolate and aggregate a step failure"
}
$repairOwnership = New-AmeBrokerUpgradeOwnership -InstalledBinaryExists $false
Remove-AmeBrokerTransactionBackup `
    -Ownership $repairOwnership `
    -BackupPath "C:\this-path-must-not-be-read\missing.previous"
if ($repairOwnership.BackupCreated) {
    throw "A missing-binary repair claimed ownership of a nonexistent backup"
}
$backupRemovalProbe = [pscustomobject]@{ Called = $false }
Remove-AmeBrokerTransactionBackup `
    -Ownership $repairOwnership `
    -BackupPath "C:\this-path-must-not-be-read\missing.previous" `
    -RemoveAction {
        $backupRemovalProbe.Called = $true
        throw "The repair path must not invoke backup removal"
    }
if ($backupRemovalProbe.Called) {
    throw "A missing-binary repair attempted to delete an unowned backup path"
}
$ownedBackup = New-AmeBrokerUpgradeOwnership -InstalledBinaryExists $true
$ownedBackup.BackupCreated = $true
Assert-Rejected `
    -Action {
        Remove-AmeBrokerTransactionBackup `
            -Ownership $ownedBackup `
            -BackupPath "C:\injected\broker.previous" `
            -PathExistsProbe { return $true } `
            -RemoveAction { throw "injected backup deletion failure" }
    } `
    -Message "The backup removal injection did not surface its failure"
if (-not $ownedBackup.BackupCreated) {
    throw "A failed backup deletion discarded transaction ownership evidence"
}

$markerAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $true
    Rules = @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            AccessControlType = "Allow"
            InheritanceFlags = 0
            PropagationFlags = 0
            IsInherited = $false
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            AccessControlType = "Allow"
            InheritanceFlags = 0
            PropagationFlags = 0
            IsInherited = $false
        }
    )
}
Assert-AmeBrokerTransactionMarkerAclFacts -Facts $markerAclFacts
$markerAclFacts.Rules[1].Sid = $script:AmeBrokerUsersSid
Assert-Rejected `
    -Action { Assert-AmeBrokerTransactionMarkerAclFacts -Facts $markerAclFacts } `
    -Message "The transaction marker ACL admitted an interactive user"
$markerAclFacts.Rules[1].Sid = $script:AmeBrokerAdministratorsSid

$parentPrestate = [pscustomobject]@{
    Path = Get-AmeProductParentDirectory
    Existed = $true
    Sddl = "O:BAG:BAD:P(A;;FA;;;BA)"
}
Assert-AmeBrokerParentPrestateRestoredFacts `
    -Prestate $parentPrestate `
    -CurrentExists $true `
    -CurrentSddl $parentPrestate.Sddl
Assert-Rejected `
    -Action {
        Assert-AmeBrokerParentPrestateRestoredFacts `
            -Prestate $parentPrestate `
            -CurrentExists $true `
            -CurrentSddl "O:BAG:BAD:P(A;;FA;;;SY)"
    } `
    -Message "The parent restore guard accepted a changed pre-existing ACL"
$newParentPrestate = [pscustomobject]@{
    Path = Get-AmeProductParentDirectory
    Existed = $false
    Sddl = $null
}
Assert-AmeBrokerParentPrestateRestoredFacts `
    -Prestate $newParentPrestate `
    -CurrentExists $false
Assert-Rejected `
    -Action {
        Assert-AmeBrokerParentPrestateRestoredFacts `
            -Prestate $newParentPrestate `
            -CurrentExists $true
    } `
    -Message "The parent restore guard accepted a leftover transaction-created parent"
$transactionState = New-AmeBrokerTransactionState `
    -Operation upgrade `
    -ParentPrestate $parentPrestate `
    -ExpectedPublisher "CN=Cedarflake Test Publisher"
$transactionState.OwnerProcessId = 4242
$transactionState.OwnerStartUtcTicks = [int64]987654321
$transactionState.StagedBinary = "$($transactionState.InstalledBinary).upgrading-4242"
$transactionState.BackupBinary = "$($transactionState.InstalledBinary).previous-4242"
Assert-AmeBrokerTransactionStateFacts -State $transactionState
if (-not (Test-AmeBrokerTransactionOwnerActive `
    -State $transactionState `
    -ProcessProbe {
        param([int]$ProcessId)
        [pscustomobject]@{ Exists = $true; StartUtcTicks = [int64]987654321 }
    })) {
    throw "A matching other-PID transaction owner was not retained"
}
if (Test-AmeBrokerTransactionOwnerActive `
    -State $transactionState `
    -ProcessProbe {
        param([int]$ProcessId)
        [pscustomobject]@{ Exists = $true; StartUtcTicks = [int64]987654322 }
    }) {
    throw "PID reuse was mistaken for the original transaction owner"
}

$oldBinaryFacts = [pscustomobject]@{
    Exists = $true
    Length = [int64]111
    Sha256 = "A" * 64
    FileIdentity = "00000001:0000000000000001"
    FinalPath = [string]$transactionState.InstalledBinary
}
$newBinaryFacts = [pscustomobject]@{
    Exists = $true
    Length = [int64]222
    Sha256 = "B" * 64
    FileIdentity = "00000001:0000000000000002"
    FinalPath = [string]$transactionState.StagedBinary
}
$transactionState.ExpectedPreviousBinary = $oldBinaryFacts
$transactionState.ExpectedNewBinary = $newBinaryFacts

$initialTreeTrace = [Collections.Generic.List[string]]::new()
$initialTreeJournal = [pscustomobject]@{ Last = $null }
$initialInstallState = New-AmeBrokerTransactionState `
    -Operation install `
    -ParentPrestate $newParentPrestate `
    -ExpectedPublisher "CN=Cedarflake Test Publisher" `
    -OwnedDirectories @(
        Get-AmeProductParentDirectory
        Get-AmeBrokerInstallDirectory
    )
Assert-Rejected `
    -Action {
        Invoke-AmeBrokerInitialTreeCreation `
            -State $initialInstallState `
            -StateWriter {
                param([psobject]$Value)
                $initialTreeTrace.Add("marker")
                $initialTreeJournal.Last = $Value |
                    ConvertTo-Json -Depth 8 |
                    ConvertFrom-Json
            } `
            -CreateTreeAction {
                $initialTreeTrace.Add("tree")
                throw "injected hard termination after tree creation"
            }
    } `
    -Message "The initial tree fault injection did not surface its failure"
if (($initialTreeTrace -join ",") -cne "marker,tree" -or
    $null -eq $initialTreeJournal.Last -or
    [string]$initialTreeJournal.Last.Phase -cne "tree_creation_pending" -or
    @($initialTreeJournal.Last.OwnedDirectories).Count -ne 2) {
    throw "Initial install did not durably own the tree before creation"
}

$uninstallSteps = @(
    [pscustomobject]@{
        Property = "UninstallProtectedTreeReady"
        Pending = "uninstall_tree_creation_pending"
        Complete = "uninstall_tree_ready"
    },
    [pscustomobject]@{
        Property = "UninstallServiceRemoved"
        Pending = "uninstall_service_removal_pending"
        Complete = "uninstall_service_removed"
    },
    [pscustomobject]@{
        Property = "UninstallBinaryRemoved"
        Pending = "uninstall_binary_removal_pending"
        Complete = "uninstall_binary_removed"
    },
    [pscustomobject]@{
        Property = "UninstallApplicationRemoved"
        Pending = "uninstall_application_removal_pending"
        Complete = "uninstall_application_removed"
    },
    [pscustomobject]@{
        Property = "UninstallInstallTreeRemoved"
        Pending = "uninstall_install_tree_removal_pending"
        Complete = "uninstall_install_tree_removed"
    },
    [pscustomobject]@{
        Property = "UninstallOwnedDirectoriesRemoved"
        Pending = "uninstall_owned_directories_removal_pending"
        Complete = "uninstall_owned_directories_removed"
    },
    [pscustomobject]@{
        Property = "UninstallParentRestored"
        Pending = "uninstall_parent_restore_pending"
        Complete = "uninstall_parent_restored"
    }
)
foreach ($uninstallStep in $uninstallSteps) {
    $completionProperty = [string]$uninstallStep.Property
    foreach ($faultBoundary in @(
        "BeforePendingAction",
        "AfterPendingAction",
        "AfterMutationAction",
        "AfterCompletedAction"
    )) {
        $state = New-AmeBrokerTransactionState `
            -Operation uninstall `
            -ParentPrestate $newParentPrestate `
            -ExpectedPublisher "CN=Cedarflake Test Publisher" `
            -OwnedDirectories @(
                Get-AmeProductParentDirectory
                Get-AmeBrokerInstallDirectory
            )
        $physical = [pscustomobject]@{ Complete = $false }
        $marker = [pscustomobject]@{
            Exists = $true
            Last = $state | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        }
        $writer = {
            param([psobject]$Value)
            $marker.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        }.GetNewClosure()
        $mutation = { $physical.Complete = $true }.GetNewClosure()
        $verify = {
            if (-not $physical.Complete) {
                throw "The injected uninstall state is incomplete"
            }
        }.GetNewClosure()
        $fault = { throw "injected uninstall $faultBoundary" }.GetNewClosure()
        $arguments = @{
            State = $state
            CompletionProperty = $completionProperty
            PendingPhase = [string]$uninstallStep.Pending
            CompletedPhase = [string]$uninstallStep.Complete
            MutationAction = $mutation
            VerifyAction = $verify
            StateWriter = $writer
        }
        $arguments[$faultBoundary] = $fault
        Assert-Rejected `
            -Action { Invoke-AmeBrokerUninstallWriteAheadStep @arguments } `
            -Message "$($uninstallStep.Property) did not surface $faultBoundary"
        if (-not $marker.Exists) {
            throw "$($uninstallStep.Property) removed the marker at $faultBoundary"
        }
        $recoveryState = $marker.Last
        Invoke-AmeBrokerUninstallWriteAheadStep `
            -State $recoveryState `
            -CompletionProperty $completionProperty `
            -PendingPhase ([string]$uninstallStep.Pending) `
            -CompletedPhase ([string]$uninstallStep.Complete) `
            -MutationAction $mutation `
            -VerifyAction $verify `
            -StateWriter $writer
        if (-not $physical.Complete -or
            -not [bool]$marker.Last.$completionProperty -or
            [string]$marker.Last.Phase -cne [string]$uninstallStep.Complete) {
            throw "$($uninstallStep.Property) did not recover idempotently after $faultBoundary"
        }
    }
}

$committedState = New-AmeBrokerTransactionState `
    -Operation uninstall `
    -ParentPrestate $newParentPrestate `
    -ExpectedPublisher "CN=Cedarflake Test Publisher"
$committedState.UninstallProtectedTreeReady = $true
$committedState.UninstallServiceRemoved = $true
$committedState.UninstallBinaryRemoved = $true
$committedState.UninstallApplicationRemoved = $true
$committedState.Phase = "uninstall_application_removed"
$committedMarker = [pscustomobject]@{ Exists = $true; Last = $null }
$committedWriter = {
    param([psobject]$Value)
    $committedMarker.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
}.GetNewClosure()
Assert-Rejected `
    -Action {
        Set-AmeBrokerUninstallCommitted `
            -State $committedState `
            -StateWriter $committedWriter `
            -AfterCommittedAction { throw "injected crash after committed marker" }
    } `
    -Message "The uninstall committed-boundary injection did not surface its failure"
if (-not $committedMarker.Exists -or
    [string]$committedMarker.Last.Phase -cne "committed") {
    throw "Committed uninstall recovery discarded its marker before final cleanup"
}

$committedState.UninstallInstallTreeRemoved = $true
$committedState.UninstallOwnedDirectoriesRemoved = $true
$committedState.UninstallParentRestored = $true
$committedState.Phase = "uninstall_parent_restored"
$finalMarker = [pscustomobject]@{ Exists = $true; Last = $committedMarker.Last }
$finalWriter = {
    param([psobject]$Value)
    $finalMarker.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
}.GetNewClosure()
$removeFinalMarker = { $finalMarker.Exists = $false }.GetNewClosure()
Assert-Rejected `
    -Action {
        Remove-AmeBrokerCompletedUninstallMarker `
            -State $committedState `
            -FinalVerifyAction {} `
            -StateWriter $finalWriter `
            -BeforeMarkerRemovalAction { throw "injected crash before marker removal" } `
            -MarkerRemovalAction $removeFinalMarker
    } `
    -Message "The uninstall final-cleanup injection did not surface its failure"
if (-not $finalMarker.Exists -or
    [string]$finalMarker.Last.Phase -cne "final_cleanup_verified") {
    throw "Final uninstall cleanup removed its marker before the verified boundary"
}
Remove-AmeBrokerCompletedUninstallMarker `
    -State $finalMarker.Last `
    -FinalVerifyAction {} `
    -StateWriter $finalWriter `
    -MarkerRemovalAction $removeFinalMarker
if ($finalMarker.Exists) {
    throw "Recovered uninstall left an orphan marker after verified final cleanup"
}

function New-UninstallTransactionFixture {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$InitiallyMissingTree
    )

    $prestate = if ($InitiallyMissingTree) { $newParentPrestate } else { $parentPrestate }
    $ownedDirectories = @()
    if ($InitiallyMissingTree) {
        $ownedDirectories = @(
            Get-AmeProductParentDirectory
            Get-AmeBrokerInstallDirectory
        )
    }
    $state = New-AmeBrokerTransactionState `
        -Operation uninstall `
        -ParentPrestate $prestate `
        -ExpectedPublisher "CN=Cedarflake Test Publisher" `
        -OwnedDirectories $ownedDirectories
    $physical = [pscustomobject]@{
        ProtectedTreeReady = -not $InitiallyMissingTree
        ServiceExists = -not $InitiallyMissingTree
        BinaryExists = -not $InitiallyMissingTree
        ApplicationExists = -not $InitiallyMissingTree
        InstallTreeExists = -not $InitiallyMissingTree
        OwnedDirectoriesExist = $false
        ParentExists = [bool]$prestate.Existed
        ParentSddl = $prestate.Sddl
    }
    $marker = [pscustomobject]@{
        Exists = $true
        Last = $state | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    }
    $writer = {
        param([psobject]$Value)
        $marker.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    }.GetNewClosure()
    $reader = {
        if (-not $marker.Exists) {
            return $null
        }
        return $marker.Last | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    }.GetNewClosure()
    $removeMarker = { $marker.Exists = $false }.GetNewClosure()
    $actions = @{
        UninstallProtectedTreeReady = [pscustomobject]@{
            MutationAction = {
                if (-not $physical.InstallTreeExists) {
                    $physical.InstallTreeExists = $true
                    $physical.OwnedDirectoriesExist = $true
                    $physical.ParentExists = $true
                    $physical.ParentSddl = "protected"
                }
                $physical.ProtectedTreeReady = $true
            }.GetNewClosure()
            VerifyAction = {
                if (-not $physical.InstallTreeExists -or
                    -not $physical.ProtectedTreeReady) {
                    throw "The simulated protected tree is not ready"
                }
            }.GetNewClosure()
        }
        UninstallServiceRemoved = [pscustomobject]@{
            MutationAction = { $physical.ServiceExists = $false }.GetNewClosure()
            VerifyAction = {
                if ($physical.ServiceExists) {
                    throw "The simulated service remains"
                }
            }.GetNewClosure()
        }
        UninstallBinaryRemoved = [pscustomobject]@{
            MutationAction = { $physical.BinaryExists = $false }.GetNewClosure()
            VerifyAction = {
                if ($physical.BinaryExists) {
                    throw "The simulated binary remains"
                }
            }.GetNewClosure()
        }
        UninstallApplicationRemoved = [pscustomobject]@{
            MutationAction = { $physical.ApplicationExists = $false }.GetNewClosure()
            VerifyAction = {
                if ($physical.ApplicationExists) {
                    throw "The simulated Application bundle remains"
                }
            }.GetNewClosure()
        }
        UninstallInstallTreeRemoved = [pscustomobject]@{
            MutationAction = {
                $physical.InstallTreeExists = $false
                $physical.ProtectedTreeReady = $false
            }.GetNewClosure()
            VerifyAction = {
                if ($physical.InstallTreeExists) {
                    throw "The simulated install tree remains"
                }
            }.GetNewClosure()
        }
        UninstallOwnedDirectoriesRemoved = [pscustomobject]@{
            MutationAction = { $physical.OwnedDirectoriesExist = $false }.GetNewClosure()
            VerifyAction = {
                if ($physical.OwnedDirectoriesExist) {
                    throw "A simulated transaction-owned ancestor remains"
                }
            }.GetNewClosure()
        }
        UninstallParentRestored = [pscustomobject]@{
            MutationAction = {
                $physical.ParentExists = [bool]$prestate.Existed
                $physical.ParentSddl = $prestate.Sddl
            }.GetNewClosure()
            VerifyAction = {
                if ($physical.ParentExists -ne [bool]$prestate.Existed -or
                    [string]$physical.ParentSddl -cne [string]$prestate.Sddl) {
                    throw "The simulated parent prestate is not exact"
                }
            }.GetNewClosure()
        }
    }
    return [pscustomobject]@{
        State = $state
        Physical = $physical
        Marker = $marker
        Writer = $writer
        Reader = $reader
        RemoveMarker = $removeMarker
        Actions = $actions
        Prestate = $prestate
    }
}

function Invoke-UninstallFixtureDeadOwnerRecovery {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Fixture
    )

    Repair-AmeBrokerInterruptedTransaction `
        -ProcessProbe {
            param([int]$ProcessId)
            [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
        } `
        -StateReader $Fixture.Reader `
        -UninstallStateWriter $Fixture.Writer `
        -UninstallMarkerRemovalAction $Fixture.RemoveMarker `
        -UninstallActionOverrides $Fixture.Actions
}

function Assert-UninstallFixtureCompleted {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Fixture,
        [Parameter(Mandatory = $true)]
        [string]$Boundary
    )

    if ($Fixture.Marker.Exists -or
        $Fixture.Physical.ServiceExists -or
        $Fixture.Physical.BinaryExists -or
        $Fixture.Physical.ApplicationExists -or
        $Fixture.Physical.InstallTreeExists -or
        $Fixture.Physical.OwnedDirectoriesExist -or
        $Fixture.Physical.ParentExists -ne [bool]$Fixture.Prestate.Existed -or
        [string]$Fixture.Physical.ParentSddl -cne [string]$Fixture.Prestate.Sddl) {
        throw "Transaction-level uninstall recovery did not restore exact prestate at $Boundary"
    }
}

$transactionUninstallSteps = @(
    Get-AmeBrokerUninstallStepDefinitions | ForEach-Object { [string]$_.Property }
)
foreach ($initiallyMissingTree in @($false, $true)) {
    foreach ($transactionStep in $transactionUninstallSteps) {
        foreach ($faultBoundary in @("BeforeMutation", "AfterMutation", "AfterCompletion")) {
            $fixture = New-UninstallTransactionFixture `
                -InitiallyMissingTree $initiallyMissingTree
            Assert-Rejected `
                -Action {
                    Complete-AmeBrokerUninstallTransaction `
                        -State $fixture.State `
                        -StateWriter $fixture.Writer `
                        -MarkerRemovalAction $fixture.RemoveMarker `
                        -ActionOverrides $fixture.Actions `
                        -FaultOperation $transactionStep `
                        -FaultBoundary $faultBoundary `
                        -FaultAction { throw "injected transaction-level uninstall crash" }
                } `
                -Message "Complete uninstall did not surface $transactionStep/$faultBoundary"
            if (-not $fixture.Marker.Exists) {
                throw "Uninstall removed its marker at $transactionStep/$faultBoundary"
            }
            try {
                Invoke-UninstallFixtureDeadOwnerRecovery -Fixture $fixture
            } catch {
                throw "Dead-owner recovery failed at " +
                    "$initiallyMissingTree/$transactionStep/$faultBoundary`: " +
                    $_.Exception.Message
            }
            Assert-UninstallFixtureCompleted `
                -Fixture $fixture `
                -Boundary "$initiallyMissingTree/$transactionStep/$faultBoundary"
        }
    }
}

$committedFixture = New-UninstallTransactionFixture -InitiallyMissingTree $false
Assert-Rejected `
    -Action {
        Complete-AmeBrokerUninstallTransaction `
            -State $committedFixture.State `
            -StateWriter $committedFixture.Writer `
            -MarkerRemovalAction $committedFixture.RemoveMarker `
            -ActionOverrides $committedFixture.Actions `
            -AfterCommittedAction { throw "injected transaction-level committed crash" }
    } `
    -Message "Complete uninstall did not surface the committed crash"
if (-not $committedFixture.Marker.Exists -or
    [string]$committedFixture.Marker.Last.Phase -cne "committed") {
    throw "Transaction-level committed crash lost its recoverable marker"
}
Invoke-UninstallFixtureDeadOwnerRecovery -Fixture $committedFixture
Assert-UninstallFixtureCompleted -Fixture $committedFixture -Boundary "committed"

$beforeFinalRemovalFixture = New-UninstallTransactionFixture -InitiallyMissingTree $false
Assert-Rejected `
    -Action {
        Complete-AmeBrokerUninstallTransaction `
            -State $beforeFinalRemovalFixture.State `
            -StateWriter $beforeFinalRemovalFixture.Writer `
            -MarkerRemovalAction $beforeFinalRemovalFixture.RemoveMarker `
            -ActionOverrides $beforeFinalRemovalFixture.Actions `
            -BeforeMarkerRemovalAction { throw "injected crash before final marker removal" }
    } `
    -Message "Complete uninstall did not surface the pre-marker-removal crash"
if (-not $beforeFinalRemovalFixture.Marker.Exists -or
    [string]$beforeFinalRemovalFixture.Marker.Last.Phase -cne "final_cleanup_verified") {
    throw "Pre-removal crash lost the verified recoverable marker"
}
Invoke-UninstallFixtureDeadOwnerRecovery -Fixture $beforeFinalRemovalFixture
Assert-UninstallFixtureCompleted -Fixture $beforeFinalRemovalFixture -Boundary "before marker removal"

$afterFinalRemovalFixture = New-UninstallTransactionFixture -InitiallyMissingTree $false
Assert-Rejected `
    -Action {
        Complete-AmeBrokerUninstallTransaction `
            -State $afterFinalRemovalFixture.State `
            -StateWriter $afterFinalRemovalFixture.Writer `
            -MarkerRemovalAction $afterFinalRemovalFixture.RemoveMarker `
            -ActionOverrides $afterFinalRemovalFixture.Actions `
            -AfterMarkerRemovalAction { throw "injected crash after final marker removal" }
    } `
    -Message "Complete uninstall did not surface the post-marker-removal crash"
Invoke-UninstallFixtureDeadOwnerRecovery -Fixture $afterFinalRemovalFixture
Assert-UninstallFixtureCompleted -Fixture $afterFinalRemovalFixture -Boundary "after marker removal"

$contradictoryFixture = New-UninstallTransactionFixture -InitiallyMissingTree $false
$contradictoryFixture.Marker.Last.UninstallProtectedTreeReady = $true
$contradictoryFixture.Marker.Last.UninstallServiceRemoved = $true
$contradictoryFixture.Marker.Last.UninstallBinaryRemoved = $true
$contradictoryFixture.Marker.Last.UninstallApplicationRemoved = $true
$contradictoryFixture.Marker.Last.Phase = "uninstall_install_tree_removal_pending"
$contradictoryFixture.Marker.Last.UninstallPendingOperation = "UninstallParentRestored"
$contradictoryFixture.Physical.ServiceExists = $false
$contradictoryFixture.Physical.BinaryExists = $false
$contradictoryFixture.Physical.ApplicationExists = $false
$contradictoryFixture.Physical.InstallTreeExists = $false
$contradictoryFixture.Physical.ProtectedTreeReady = $false
Assert-Rejected `
    -Action { Invoke-UninstallFixtureDeadOwnerRecovery -Fixture $contradictoryFixture } `
    -Message "Dead-owner recovery accepted contradictory uninstall progress"
if (-not $contradictoryFixture.Marker.Exists) {
    throw "Fail-closed contradictory recovery removed its recoverable marker"
}

foreach ($moveCase in @(
    [pscustomobject]@{
        Name = "installed_to_backup"
        Source = [string]$transactionState.InstalledBinary
        Destination = [string]$transactionState.BackupBinary
        Expected = $oldBinaryFacts
        NextPhase = "backup_created"
    },
    [pscustomobject]@{
        Name = "staged_to_installed"
        Source = [string]$transactionState.StagedBinary
        Destination = [string]$transactionState.InstalledBinary
        Expected = $newBinaryFacts
        NextPhase = "installed_binary"
    }
)) {
    foreach ($faultBoundary in @(
        "BeforePendingAction",
        "AfterPendingAction",
        "AfterMoveAction",
        "AfterAppliedAction"
    )) {
        $state = New-AmeBrokerTransactionState `
            -Operation upgrade `
            -ParentPrestate $parentPrestate `
            -ExpectedPublisher "CN=Cedarflake Test Publisher"
        $state.OwnerProcessId = 4242
        $state.OwnerStartUtcTicks = [int64]987654321
        $state.StagedBinary = "$($state.InstalledBinary).upgrading-4242"
        $state.BackupBinary = "$($state.InstalledBinary).previous-4242"
        $state.ExpectedPreviousBinary = $oldBinaryFacts
        $state.ExpectedNewBinary = $newBinaryFacts
        $physical = @{}
        foreach ($path in @(
            [string]$state.InstalledBinary,
            [string]$state.StagedBinary,
            [string]$state.BackupBinary
        )) {
            $physical[[IO.Path]::GetFullPath($path)] = New-AmeBrokerAbsentBinaryFacts
        }
        $expectedSource = if ($moveCase.Name -ceq "installed_to_backup") {
            $oldBinaryFacts
        } else {
            $newBinaryFacts
        }
        $physical[[IO.Path]::GetFullPath([string]$moveCase.Source)] = $expectedSource
        $journal = [pscustomobject]@{
            Last = $null
            WriteCount = 0
        }
        $writer = {
            param([psobject]$Value)
            $journal.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
            $journal.WriteCount += 1
        }.GetNewClosure()
        $probe = {
            param([string]$Path, [string]$Publisher)
            return $physical[[IO.Path]::GetFullPath($Path)]
        }.GetNewClosure()
        $move = {
            param([string]$Source, [string]$Destination)
            $sourceKey = [IO.Path]::GetFullPath($Source)
            $destinationKey = [IO.Path]::GetFullPath($Destination)
            if (-not [bool]$physical[$sourceKey].Exists -or
                [bool]$physical[$destinationKey].Exists) {
                throw "The injected move observed an invalid physical state"
            }
            $physical[$destinationKey] = $physical[$sourceKey]
            $physical[$sourceKey] = [pscustomobject]@{
                Exists = $false
                Length = [int64]0
                Sha256 = $null
                FileIdentity = $null
                FinalPath = $null
            }
        }.GetNewClosure()
        $fault = { throw "injected $faultBoundary" }.GetNewClosure()
        $arguments = @{
            State = $state
            SourcePath = [string]$moveCase.Source
            DestinationPath = [string]$moveCase.Destination
            ExpectedSource = $expectedSource
            ExpectedDestinationBefore = New-AmeBrokerAbsentBinaryFacts
            NextPhase = [string]$moveCase.NextPhase
            StateWriter = $writer
            FactProbe = $probe
            MoveAction = $move
        }
        $arguments[$faultBoundary] = $fault
        Assert-Rejected `
            -Action { Invoke-AmeBrokerWriteAheadMove @arguments } `
            -Message "$($moveCase.Name) did not surface $faultBoundary"
        if ($faultBoundary -ceq "BeforePendingAction") {
            if ($journal.WriteCount -ne 0 -or
                -not [bool]$physical[[IO.Path]::GetFullPath($moveCase.Source)].Exists) {
                throw "$($moveCase.Name) mutated before its pending marker"
            }
            continue
        }
        $recoveryState = $journal.Last
        Resolve-AmeBrokerPendingMove `
            -State $recoveryState `
            -StateWriter $writer `
            -FactProbe $probe `
            -MoveAction $move
        $sourceAfter = $physical[[IO.Path]::GetFullPath($moveCase.Source)]
        $destinationAfter = $physical[[IO.Path]::GetFullPath($moveCase.Destination)]
        if ([bool]$sourceAfter.Exists -or
            -not (Test-AmeBrokerBinaryFactsMatch `
                -Expected $expectedSource `
                -Actual $destinationAfter) -or
            $null -ne $journal.Last.PendingIntent -or
            @($journal.Last.AppliedOperations).Count -ne 1 -or
            [string]$journal.Last.Phase -cne [string]$moveCase.NextPhase) {
            throw "$($moveCase.Name) did not recover idempotently after $faultBoundary"
        }
    }
}

$transactionState.StagedBinary = "C:\Users\Public\replacement.exe"
Assert-Rejected `
    -Action { Assert-AmeBrokerTransactionStateFacts -State $transactionState } `
    -Message "The transaction marker admitted an external staging path"

$noFollowRoot = "C:\guardrail-root"
$ordinaryEntry = [pscustomobject]@{
    FullName = "C:\guardrail-root\ordinary"
    IsReparsePoint = $false
}
Assert-AmeBrokerNoFollowEntryFacts -Entries @($ordinaryEntry) -RootPath $noFollowRoot
$junctionEntry = [pscustomobject]@{
    FullName = "C:\guardrail-root\child-junction"
    IsReparsePoint = $true
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerNoFollowEntryFacts -Entries @($junctionEntry) -RootPath $noFollowRoot
    } `
    -Message "The no-follow walker admitted a descendant junction"
$escapedEntry = [pscustomobject]@{
    FullName = "C:\external\escaped"
    IsReparsePoint = $false
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerNoFollowEntryFacts -Entries @($escapedEntry) -RootPath $noFollowRoot
    } `
    -Message "The no-follow walker admitted an escaped descendant"

$creationTrace = [System.Collections.Generic.List[string]]::new()
$serviceCreated = $false
Assert-Rejected `
    -Action {
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath "C:\Program Files\Cedarflake Ame\Journal Broker\cedarflake_ame_journal_broker.exe" `
            -IdentityManifest $identityManifest `
            -Create `
            -CreatedState ([ref]$serviceCreated) `
            -ScInvoker {
                param([string[]]$Arguments)
                $creationTrace.Add($Arguments[0])
                if ($Arguments[0] -ceq "sidtype") {
                    throw "injected post-create failure"
                }
            }
    } `
    -Message "The service creation injection did not surface its post-create failure"
if (-not $serviceCreated -or ($creationTrace -join ",") -cne "create,sidtype") {
    throw "Service creation did not retain ownership after a post-create failure"
}

$deleteProbe = [pscustomobject]@{ Calls = 0 }
Wait-AmeBrokerServiceDeletion `
    -TimeoutMilliseconds 100 `
    -PollMilliseconds 1 `
    -ServiceExistsProbe {
        $deleteProbe.Calls += 1
        return $deleteProbe.Calls -lt 3
    }
if ($deleteProbe.Calls -ne 3) {
    throw "The service deletion wait did not observe disappearance deterministically"
}

$programFilesFixture = Get-AmeBrokerProgramFilesX64
$expectedInstallDirectory = Join-Path $programFilesFixture "Cedarflake Ame\Journal Broker"
$expectedApplicationDirectory = Join-Path $programFilesFixture "Cedarflake Ame\Application"
Assert-AmeBrokerInstallDirectoryFacts `
    -CandidatePath $expectedInstallDirectory `
    -TrustedProgramFilesPath $programFilesFixture
foreach ($invalidInstallDirectory in @(
    $programFilesFixture,
    (Join-Path $programFilesFixture "Cedarflake Ame"),
    "C:\Users\Public\Cedarflake Ame",
    "C:\"
)) {
    Assert-Rejected `
        -Action {
            Assert-AmeBrokerInstallDirectoryFacts `
                -CandidatePath $invalidInstallDirectory `
                -TrustedProgramFilesPath $programFilesFixture
        } `
        -Message "The broker guard accepted an unowned install directory"
}
Assert-AmeApplicationInstallDirectoryFacts `
    -CandidatePath $expectedApplicationDirectory `
    -TrustedProgramFilesPath $programFilesFixture
foreach ($invalidApplicationDirectory in @(
    $programFilesFixture,
    (Join-Path $programFilesFixture "Cedarflake Ame"),
    "C:\Users\Public\Cedarflake Ame\Application",
    "C:\"
)) {
    Assert-Rejected `
        -Action {
            Assert-AmeApplicationInstallDirectoryFacts `
                -CandidatePath $invalidApplicationDirectory `
                -TrustedProgramFilesPath $programFilesFixture
        } `
        -Message "The application guard accepted a non-fixed directory"
}

$originalProgramFiles = $env:ProgramFiles
try {
    $env:ProgramFiles = "C:\Users\Public\Untrusted Program Files"
    $knownFolderInstallDirectory = Get-AmeBrokerInstallDirectory
    if (-not $knownFolderInstallDirectory.Equals(
        $expectedInstallDirectory,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "The broker install directory trusted a poisoned ProgramFiles environment value"
    }
} finally {
    $env:ProgramFiles = $originalProgramFiles
}
Assert-AmeBrokerFixedInstallTree -InstallDirectory $expectedInstallDirectory | Out-Null

function New-TestInstallTreeFacts {
    param(
        [string]$FinalPath = "C:\Program Files",
        [bool]$IsReparsePoint = $false,
        [string]$OwnerSid = $script:AmeBrokerTrustedInstallerSid,
        [uint64]$UntrustedRights = [uint64]0x001200A9,
        [bool]$IsVolumeRoot = $false
    )

    $acl = [pscustomobject]@{
        OwnerSid = $OwnerSid
        AreAccessRulesProtected = $true
        Rules = @(
            [pscustomobject]@{
                Sid = "S-1-5-32-545"
                Identity = "BUILTIN\Users"
                Rights = $UntrustedRights
                AccessControlType = "Allow"
                InheritanceFlags = 0
                PropagationFlags = 0
                IsInherited = $false
            }
        )
    }
    return [pscustomobject]@{
        TrustedProgramFilesPath = "C:\Program Files"
        InstallDirectory = "C:\Program Files\Cedarflake Ame\Journal Broker"
        ExpectedPaths = @(
            "C:\",
            "C:\Program Files",
            "C:\Program Files\Cedarflake Ame",
            "C:\Program Files\Cedarflake Ame\Journal Broker"
        )
        ExistingEntries = @(
            [pscustomobject]@{
                ExpectedPath = "C:\Program Files"
                FinalPath = $FinalPath
                IsDirectory = $true
                IsReparsePoint = $IsReparsePoint
                IsVolumeRoot = $IsVolumeRoot
                Acl = $acl
            }
        )
        FirstMissingIndex = 2
    }
}

Assert-AmeBrokerInstallTreeFacts -Facts (New-TestInstallTreeFacts)
foreach ($unsafeTree in @(
    (New-TestInstallTreeFacts -FinalPath "C:\Users\Public"),
    (New-TestInstallTreeFacts -IsReparsePoint $true),
    (New-TestInstallTreeFacts -OwnerSid "S-1-5-32-545"),
    (New-TestInstallTreeFacts -UntrustedRights ([uint64]0x2)),
    (New-TestInstallTreeFacts -UntrustedRights ([uint64]0x40) -IsVolumeRoot $true)
)) {
    Assert-Rejected `
        -Action { Assert-AmeBrokerInstallTreeFacts -Facts $unsafeTree } `
        -Message "The broker physical install-tree guard accepted an unsafe OS fact"
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerInstallTreeFacts `
            -Facts (New-TestInstallTreeFacts) `
            -RequireInstallDirectory
    } `
    -Message "The broker install-tree guard accepted a missing final directory"

$provisionalAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $true
    Rules = @(
        [pscustomobject]@{
            Sid = $script:AmeBrokerSystemSid
            Rights = [uint64]2032127
            AccessControlType = "Allow"
            InheritanceFlags = 3
            PropagationFlags = 0
            IsInherited = $false
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerAdministratorsSid
            Rights = [uint64]2032127
            AccessControlType = "Allow"
            InheritanceFlags = 3
            PropagationFlags = 0
            IsInherited = $false
        },
        [pscustomobject]@{
            Sid = $script:AmeBrokerUsersSid
            Rights = [uint64]131241
            AccessControlType = "Allow"
            InheritanceFlags = 3
            PropagationFlags = 0
            IsInherited = $false
        }
    )
}
Assert-AmeBrokerExactAclFacts `
    -Facts $provisionalAclFacts `
    -Kind "Directory" `
    -IncludeServiceSid $false
$emptyOrphanFacts = [pscustomobject]@{
    DirectoryExists = $true
    ServiceExists = $false
    MarkerExists = $false
    BinaryExists = $false
    EntryCount = 0
    Acl = $provisionalAclFacts
}
Assert-AmeBrokerAdoptableOrphanFacts -Facts $emptyOrphanFacts
foreach ($unsafeOrphan in @(
    [pscustomobject]@{
        DirectoryExists = $true
        ServiceExists = $false
        MarkerExists = $false
        BinaryExists = $false
        EntryCount = 1
        Acl = $provisionalAclFacts
    },
    [pscustomobject]@{
        DirectoryExists = $true
        ServiceExists = $true
        MarkerExists = $false
        BinaryExists = $false
        EntryCount = 0
        Acl = $provisionalAclFacts
    },
    [pscustomobject]@{
        DirectoryExists = $true
        ServiceExists = $false
        MarkerExists = $false
        BinaryExists = $false
        EntryCount = 0
        Acl = [pscustomobject]@{
            OwnerSid = $script:AmeBrokerUsersSid
            GroupSid = $script:AmeBrokerAdministratorsSid
            AreAccessRulesProtected = $true
            Rules = $provisionalAclFacts.Rules
        }
    }
)) {
    Assert-Rejected `
        -Action { Assert-AmeBrokerAdoptableOrphanFacts -Facts $unsafeOrphan } `
        -Message "The installer adopted a nonempty, active, or incorrectly protected orphan"
}
$applicationAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $true
    Rules = @($provisionalAclFacts.Rules)
}
Assert-AmeApplicationExactAclFacts -Facts $applicationAclFacts -Kind "Directory"
$applicationAclFacts.Rules[2].Rights = [uint64]2032127
Assert-Rejected `
    -Action { Assert-AmeApplicationExactAclFacts -Facts $applicationAclFacts -Kind "Directory" } `
    -Message "The application ACL guard accepted writable BUILTIN Users"
$applicationAclFacts.Rules[2].Rights = [uint64]131241
$overbroadAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $true
    Rules = @($provisionalAclFacts.Rules) + @(
        [pscustomobject]@{
            Sid = "S-1-5-32-545"
            Rights = [uint64]2
            AccessControlType = "Allow"
            InheritanceFlags = 3
            PropagationFlags = 0
            IsInherited = $false
        }
    )
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerExactAclFacts `
            -Facts $overbroadAclFacts `
            -Kind "Directory" `
            -IncludeServiceSid $false
    } `
    -Message "The broker exact ACL guard accepted an extra writable identity"
$wrongOwnerAclFacts = [pscustomobject]@{
    OwnerSid = "S-1-5-32-545"
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $true
    Rules = $provisionalAclFacts.Rules
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerExactAclFacts `
            -Facts $wrongOwnerAclFacts `
            -Kind "Directory" `
            -IncludeServiceSid $false
    } `
    -Message "The broker exact ACL guard accepted an untrusted owner"
$unprotectedAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerAdministratorsSid
    AreAccessRulesProtected = $false
    Rules = $provisionalAclFacts.Rules
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerExactAclFacts `
            -Facts $unprotectedAclFacts `
            -Kind "Directory" `
            -IncludeServiceSid $false
    } `
    -Message "The broker exact ACL guard accepted inherited parent authority"
$wrongGroupAclFacts = [pscustomobject]@{
    OwnerSid = $script:AmeBrokerAdministratorsSid
    GroupSid = $script:AmeBrokerUsersSid
    AreAccessRulesProtected = $true
    Rules = $provisionalAclFacts.Rules
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerExactAclFacts `
            -Facts $wrongGroupAclFacts `
            -Kind "Directory" `
            -IncludeServiceSid $false
    } `
    -Message "The broker exact ACL guard accepted an untrusted group"

Assert-AmeBrokerServiceDaclFacts -ActualSddl $script:AmeBrokerServiceDacl
Assert-AmeBrokerServiceSidFacts -ActualSid (Get-AmeBrokerCalculatedServiceSid)
Assert-Rejected `
    -Action {
        Assert-AmeBrokerServiceDaclFacts `
            -ActualSddl ($script:AmeBrokerServiceDacl + "(A;;CC;;;WD)")
    } `
    -Message "The broker exact service DACL guard accepted an extra ACE"
Assert-Rejected `
    -Action { Assert-AmeBrokerServiceSidFacts -ActualSid "S-1-5-80-1-2-3-4-5" } `
    -Message "The broker service SID guard accepted a mismatched service identity"

$plan = Get-AmeBrokerServicePlan
if (
    $plan.ServiceType -cne "own" -or
    $plan.StartMode -cne "demand" -or
    $plan.ServiceAccount -cne "LocalSystem" -or
    $plan.ServiceSidType -cne "restricted"
) {
    throw "The broker service plan expanded its process, start, account, or SID boundary"
}
if (
    $plan.RequiredPrivileges.Count -ne 1 -or
    $plan.RequiredPrivileges[0] -cne "SeManageVolumePrivilege"
) {
    throw "The broker service plan expanded its privilege allowlist"
}
if (
    -not $plan.ServiceDacl.Contains("(A;;CCLCRPLO;;;IU)") -or
    $plan.ServiceDacl.Contains("(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;IU)")
) {
    throw "The broker service DACL does not limit interactive users to query-config/status/start/interrogate"
}

$lifecycleScripts = @(
    "release_install_journal_broker.ps1",
    "release_repair_journal_broker.ps1",
    "release_upgrade_journal_broker.ps1",
    "release_stop_journal_broker.ps1",
    "release_uninstall_journal_broker.ps1"
)
$forbiddenPattern = "FSCTL_(CREATE|DELETE)_USN_JOURNAL|fsutil|local-primary|cloud-primary|sqlite|catalog"
foreach ($scriptName in $lifecycleScripts) {
    $scriptPath = Join-Path $PSScriptRoot $scriptName
    if (-not (Test-Path -LiteralPath $scriptPath -PathType Leaf)) {
        throw "The broker lifecycle is missing $scriptName"
    }
    $scriptText = Get-Content -LiteralPath $scriptPath -Raw -Encoding UTF8
    if ($scriptText -notmatch "SupportsShouldProcess") {
        throw "$scriptName must retain a WhatIf-capable mutation boundary"
    }
    if ($scriptText -match $forbiddenPattern) {
        throw "$scriptName crosses the service, source, journal, or catalog lifecycle boundary"
    }
    if ($scriptText -match '\$env:ProgramFiles') {
        throw "$scriptName must not trust the ProgramFiles environment value"
    }
    if ($scriptText -match 'Split-Path\s+-LiteralPath[^\r\n]+-Parent') {
        throw "$scriptName uses the ambiguous Windows PowerShell 5.1 Split-Path parameter set"
    }
}
foreach ($recoveringScript in @(
    "release_install_journal_broker.ps1",
    "release_upgrade_journal_broker.ps1",
    "release_uninstall_journal_broker.ps1"
)) {
    $recoveryText = Get-Content `
        -LiteralPath (Join-Path $PSScriptRoot $recoveringScript) `
        -Raw `
        -Encoding UTF8
    if ($recoveryText -notmatch 'Repair-AmeBrokerInterruptedTransaction') {
        throw "$recoveringScript must recover a protected prior transaction before mutation"
    }
    if ($recoveryText -match 'Remove-Item[^\r\n]+-Recurse') {
        throw "$recoveringScript must not use recursive deletion for transaction-owned trees"
    }
}
$installScriptText = Get-Content `
    -LiteralPath (Join-Path $PSScriptRoot "release_install_journal_broker.ps1") `
    -Raw `
    -Encoding UTF8
if (
    $installScriptText -notmatch '\[string\]\$ApplicationBundlePath' -or
    $installScriptText -notmatch 'Install-AmeApplicationBundle'
) {
    throw "The broker installer must install the complete protected Ame application bundle"
}
$uninstallScriptText = Get-Content `
    -LiteralPath (Join-Path $PSScriptRoot "release_uninstall_journal_broker.ps1") `
    -Raw `
    -Encoding UTF8
$uninstallMarkerIndex = $uninstallScriptText.IndexOf(
    "Write-AmeBrokerTransactionState -State `$transaction",
    [StringComparison]::Ordinal
)
$uninstallCompletionIndex = $uninstallScriptText.IndexOf(
    "Complete-AmeBrokerUninstallTransaction -State `$transaction",
    [StringComparison]::Ordinal
)
if ($uninstallMarkerIndex -lt 0 -or
    $uninstallCompletionIndex -le $uninstallMarkerIndex -or
    $uninstallScriptText -notmatch 'Assert-AmeApplicationInstallation') {
    throw "Uninstall must journal before its first tree mutation and own Application cleanup"
}

$commonText = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "release_journal_broker_common.ps1"),
    [System.Text.Encoding]::UTF8
)
foreach ($requiredContract in @(
    "type= own",
    "start= demand",
    "obj= LocalSystem",
    "sidtype",
    "restricted",
    "SeManageVolumePrivilege",
    "Get-AuthenticodeSignature",
    "AreAccessRulesProtected",
    "SHGetKnownFolderPath",
    "GetFinalPathNameByHandleW",
    "--protocol-info",
    "Get-FileHash",
    "signer_sha256",
    "client_path_utf16le",
    "client_sha256",
    "client_signer_sha256",
    "AmeApplicationProtectedDirectorySddl",
    "journal-broker-transaction-v2.json",
    "MoveFileWriteThrough",
    "Get-AmeBrokerNoFollowTreeEntries",
    "UninstallPendingOperation",
    "Assert-AmeBrokerUninstallProgressFacts",
    "Test-AmeBrokerUninstallPostconditionSuperseded",
    "Invoke-AmeBrokerUninstallWriteAheadStep",
    "Complete-AmeBrokerUninstallTransaction",
    "Set-AmeBrokerUninstallCommitted",
    "Remove-AmeBrokerCompletedUninstallMarker",
    "description"
)) {
    if ($commonText.IndexOf($requiredContract, [System.StringComparison]::Ordinal) -lt 0) {
        throw "The broker lifecycle common layer is missing $requiredContract"
    }
}
if ($commonText -match '\$env:ProgramFiles') {
    throw "The broker lifecycle must not trust the ProgramFiles environment value"
}
if ($commonText -match 'Split-Path\s+-LiteralPath[^\r\n]+-Parent') {
    throw "The broker common layer uses the ambiguous Windows PowerShell 5.1 Split-Path parameter set"
}
if ($commonText -match "FSCTL_(CREATE|DELETE)_USN_JOURNAL|fsutil") {
    throw "The broker installer must not configure or mutate the USN journal"
}

$candidateText = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "release_verify_candidate.ps1"),
    [System.Text.Encoding]::UTF8
)
foreach ($requiredSigningInput in @(
    "ExpectedBrokerPublisher",
    "SignedBrokerBinaryPath",
    "SignedApplicationBundlePath"
)) {
    if (
        $candidateText.IndexOf(
            "[string]`$$requiredSigningInput",
            [System.StringComparison]::Ordinal
        ) -lt 0
    ) {
        throw "Release candidate verification is missing $requiredSigningInput"
    }
}
if (($candidateText -split '\[Parameter\(Mandatory = \$true\)\]').Count -lt 4) {
    throw "Release candidate signing inputs must remain mandatory and fail closed"
}

$portablePackageText = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "release_package_portable_windows.ps1"),
    [System.Text.Encoding]::UTF8
)
if (
    $portablePackageText -notmatch "Assert-AmeBrokerBinary" -or
    $portablePackageText -notmatch "ExpectedBrokerPublisher" -or
    $portablePackageText -notmatch "release_verify_portable_signatures.ps1"
) {
    throw "Portable packaging must verify the signed broker publisher before archiving"
}

$windowsReleaseText = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "release_verify_windows.ps1"),
    [System.Text.Encoding]::UTF8
)
if (
    $windowsReleaseText -match 'flutterExecutable build windows' -or
    $windowsReleaseText -match 'Copy-Item -LiteralPath \$builtBroker' -or
    $windowsReleaseText -notmatch 'Assert-AmeApplicationBundleSource'
) {
    throw "Windows release verification must consume an immutable signed application bundle"
}

$qualityWorkflowText = [System.IO.File]::ReadAllText(
    (Join-Path $repositoryRoot ".github\workflows\quality_gate_windows.yml"),
    [System.Text.Encoding]::UTF8
)
$candidateWorkflowText = [System.IO.File]::ReadAllText(
    (Join-Path $repositoryRoot ".github\workflows\release_candidate_windows.yml"),
    [System.Text.Encoding]::UTF8
)
$publishedWorkflowText = [System.IO.File]::ReadAllText(
    (Join-Path $repositoryRoot ".github\workflows\release_verify_published.yml"),
    [System.Text.Encoding]::UTF8
)
foreach ($requiredReleaseContract in @(
    "windows_signing_pfx_base64",
    "Release gate requires production Windows signing credentials",
    "SignedApplicationBundlePath",
    "Remove-Item -LiteralPath `$certificatePath -Force",
    "Temporary signing PFX still exists after cleanup",
    "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02"
)) {
    if ($qualityWorkflowText.IndexOf(
        $requiredReleaseContract,
        [System.StringComparison]::Ordinal
    ) -lt 0) {
        throw "Windows release workflow is missing $requiredReleaseContract"
    }
}
foreach ($forbiddenReleaseContract in @(
    "New-SelfSignedCertificate",
    "Cert:\CurrentUser\Root",
    "Cert:\CurrentUser\TrustedPublisher",
    "Export-PfxCertificate"
)) {
    if ($qualityWorkflowText.IndexOf(
        $forbiddenReleaseContract,
        [System.StringComparison]::Ordinal
    ) -ge 0) {
        throw "Windows release workflow retained a disposable signing or trust path"
    }
}
if (
    $candidateWorkflowText -notmatch 'actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093' -or
    $candidateWorkflowText -notmatch 'needs\.release_windows\.outputs\.signed_bundle_artifact' -or
    $candidateWorkflowText -notmatch 'Assert-AmeApplicationBundleSource' -or
    $candidateWorkflowText -notmatch 'Assert-AmeBrokerBinary' -or
    $candidateWorkflowText -match 'flutter build windows --release'
) {
    throw "Portable publication must consume the signed candidate artifact without rebuilding"
}
if (
    $publishedWorkflowText -notmatch 'release_verify_portable_signatures.ps1' -or
    $publishedWorkflowText -notmatch 'Published verification requires the exact Authenticode publisher'
) {
    throw "Published portable verification must revalidate embedded signatures and publisher"
}
$portableGuardrailText = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "release_test_portable_archive.ps1"),
    [System.Text.Encoding]::UTF8
)
if (
    $portableGuardrailText -notmatch 'accepted the wrong publisher' -or
    $portableGuardrailText -notmatch 'accepted a tampered signed application'
) {
    throw "Portable release guardrails must reject publisher substitution and artifact tampering"
}

Write-Output "journal_broker_installer_guardrails_passed"
