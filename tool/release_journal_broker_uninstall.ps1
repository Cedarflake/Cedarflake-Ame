function Get-AmeBrokerUninstallStepDefinitions {
    return @(
        [pscustomobject]@{
            Property = "UninstallProtectedTreeReady"
            Pending = "uninstall_tree_creation_pending"
            Completed = "uninstall_tree_ready"
        },
        [pscustomobject]@{
            Property = "UninstallServiceRemoved"
            Pending = "uninstall_service_removal_pending"
            Completed = "uninstall_service_removed"
        },
        [pscustomobject]@{
            Property = "UninstallBinaryRemoved"
            Pending = "uninstall_binary_removal_pending"
            Completed = "uninstall_binary_removed"
        },
        [pscustomobject]@{
            Property = "UninstallApplicationRemoved"
            Pending = "uninstall_application_removal_pending"
            Completed = "uninstall_application_removed"
        },
        [pscustomobject]@{
            Property = "UninstallInstallTreeRemoved"
            Pending = "uninstall_install_tree_removal_pending"
            Completed = "uninstall_install_tree_removed"
        },
        [pscustomobject]@{
            Property = "UninstallOwnedDirectoriesRemoved"
            Pending = "uninstall_owned_directories_removal_pending"
            Completed = "uninstall_owned_directories_removed"
        },
        [pscustomobject]@{
            Property = "UninstallParentRestored"
            Pending = "uninstall_parent_restore_pending"
            Completed = "uninstall_parent_restored"
        }
    )
}

function Assert-AmeBrokerUninstallProgressFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction has uninstall progress"
    }
    $steps = @(Get-AmeBrokerUninstallStepDefinitions)
    $phase = [string]$State.Phase
    $pendingOperation = [string]$State.UninstallPendingOperation
    $expectedCompletedCount = $null
    $expectedPendingOperation = ""
    if ($phase -ceq "prepared") {
        $expectedCompletedCount = 0
    } elseif ($phase -ceq "committed") {
        $expectedCompletedCount = 4
    } elseif ($phase -ceq "final_cleanup_verified") {
        $expectedCompletedCount = $steps.Count
    } else {
        for ($index = 0; $index -lt $steps.Count; $index += 1) {
            if ($phase -ceq [string]$steps[$index].Pending) {
                $expectedCompletedCount = $index
                $expectedPendingOperation = [string]$steps[$index].Property
                break
            }
            if ($phase -ceq [string]$steps[$index].Completed) {
                $expectedCompletedCount = $index + 1
                break
            }
        }
    }
    if ($null -eq $expectedCompletedCount -or
        $pendingOperation -cne $expectedPendingOperation) {
        throw "The uninstall marker phase and pending operation are inconsistent"
    }
    for ($index = 0; $index -lt $steps.Count; $index += 1) {
        $expected = $index -lt $expectedCompletedCount
        $propertyName = [string]$steps[$index].Property
        $actual = [bool]$State.$propertyName
        if ($actual -ne $expected) {
            throw "The uninstall marker contains non-prefix completion state"
        }
    }
}

function Test-AmeBrokerUninstallPostconditionSuperseded {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [string]$CompletionProperty
    )

    if ($CompletionProperty -cne "UninstallProtectedTreeReady") {
        return $false
    }
    if ([bool]$State.UninstallInstallTreeRemoved) {
        return $true
    }
    return [string]$State.UninstallPendingOperation -in @(
        "UninstallInstallTreeRemoved",
        "UninstallOwnedDirectoriesRemoved",
        "UninstallParentRestored"
    )
}

function Resolve-AmeBrokerUninstallActionPair {
    param(
        [hashtable]$Overrides,
        [Parameter(Mandatory = $true)]
        [string]$CompletionProperty,
        [Parameter(Mandatory = $true)]
        [scriptblock]$DefaultMutationAction,
        [Parameter(Mandatory = $true)]
        [scriptblock]$DefaultVerifyAction
    )

    if ($null -eq $Overrides -or -not $Overrides.ContainsKey($CompletionProperty)) {
        return [pscustomobject]@{
            MutationAction = $DefaultMutationAction
            VerifyAction = $DefaultVerifyAction
        }
    }
    $override = $Overrides[$CompletionProperty]
    if ($null -eq $override -or
        $override.MutationAction -isnot [scriptblock] -or
        $override.VerifyAction -isnot [scriptblock]) {
        throw "The uninstall action override for $CompletionProperty is invalid"
    }
    return $override
}

function Get-AmeBrokerUninstallFaultAction {
    param(
        [string]$FaultOperation,
        [string]$FaultBoundary,
        [Parameter(Mandatory = $true)]
        [string]$CurrentOperation,
        [Parameter(Mandatory = $true)]
        [string]$CurrentBoundary,
        [scriptblock]$FaultAction = {}
    )

    if ($FaultOperation -ceq $CurrentOperation -and
        $FaultBoundary -ceq $CurrentBoundary) {
        return $FaultAction
    }
    return {}
}

function Invoke-AmeBrokerUninstallWriteAheadStep {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [ValidateSet(
            "UninstallProtectedTreeReady",
            "UninstallServiceRemoved",
            "UninstallBinaryRemoved",
            "UninstallApplicationRemoved",
            "UninstallInstallTreeRemoved",
            "UninstallOwnedDirectoriesRemoved",
            "UninstallParentRestored"
        )]
        [string]$CompletionProperty,
        [Parameter(Mandatory = $true)]
        [string]$PendingPhase,
        [Parameter(Mandatory = $true)]
        [string]$CompletedPhase,
        [Parameter(Mandatory = $true)]
        [scriptblock]$MutationAction,
        [Parameter(Mandatory = $true)]
        [scriptblock]$VerifyAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$BeforePendingAction = {},
        [scriptblock]$AfterPendingAction = {},
        [scriptblock]$AfterMutationAction = {},
        [scriptblock]$AfterCompletedAction = {},
        [scriptblock]$PostconditionSupersededAction = { $false }
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction may execute uninstall steps"
    }
    if ([bool]$State.$CompletionProperty) {
        if ([bool](& $PostconditionSupersededAction)) {
            return
        }
        & $VerifyAction
        return
    }
    & $BeforePendingAction
    $State.Phase = $PendingPhase
    $State.UninstallPendingOperation = $CompletionProperty
    & $StateWriter $State
    & $AfterPendingAction
    & $MutationAction
    & $AfterMutationAction
    & $VerifyAction
    $State.$CompletionProperty = $true
    $State.UninstallPendingOperation = $null
    $State.Phase = $CompletedPhase
    & $StateWriter $State
    & $AfterCompletedAction
}

function Assert-AmeBrokerParentPrestateRestored {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Prestate
    )

    $parent = Get-AmeProductParentDirectory
    $exists = Test-Path -LiteralPath $parent -PathType Container
    $isReparsePoint = $false
    $sddl = $null
    if ($exists) {
        $item = Get-Item -LiteralPath $parent -Force
        $isReparsePoint = [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
        if (-not $isReparsePoint) {
            $sddl = [string](Get-Acl -LiteralPath $parent).Sddl
        }
    }
    Assert-AmeBrokerParentPrestateRestoredFacts `
        -Prestate $Prestate `
        -CurrentExists $exists `
        -CurrentIsReparsePoint $isReparsePoint `
        -CurrentSddl $sddl
}

function Set-AmeBrokerUninstallCommitted {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$AfterCommittedAction = {}
    )

    Assert-AmeBrokerUninstallProgressFacts -State $State
    if ([string]$State.Operation -cne "uninstall" -or
        -not [bool]$State.UninstallProtectedTreeReady -or
        -not [bool]$State.UninstallServiceRemoved -or
        -not [bool]$State.UninstallBinaryRemoved -or
        -not [bool]$State.UninstallApplicationRemoved) {
        throw "Uninstall cannot commit before every core removal is verified"
    }
    if ([bool]$State.UninstallInstallTreeRemoved -or
        [bool]$State.UninstallOwnedDirectoriesRemoved -or
        [bool]$State.UninstallParentRestored -or
        [string]$State.UninstallPendingOperation -in @(
            "UninstallInstallTreeRemoved",
            "UninstallOwnedDirectoriesRemoved",
            "UninstallParentRestored"
        )) {
        return
    }
    $State.UninstallPendingOperation = $null
    $State.Phase = "committed"
    & $StateWriter $State
    & $AfterCommittedAction
}

function Remove-AmeBrokerCompletedUninstallMarker {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [scriptblock]$FinalVerifyAction,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$BeforeMarkerRemovalAction = {},
        [scriptblock]$MarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [scriptblock]$AfterMarkerRemovalAction = {}
    )

    Assert-AmeBrokerUninstallProgressFacts -State $State
    if ([string]$State.Operation -cne "uninstall" -or
        -not [bool]$State.UninstallProtectedTreeReady -or
        -not [bool]$State.UninstallServiceRemoved -or
        -not [bool]$State.UninstallBinaryRemoved -or
        -not [bool]$State.UninstallApplicationRemoved -or
        -not [bool]$State.UninstallInstallTreeRemoved -or
        -not [bool]$State.UninstallOwnedDirectoriesRemoved -or
        -not [bool]$State.UninstallParentRestored) {
        throw "Uninstall cannot remove its marker before every final action is verified"
    }
    & $FinalVerifyAction
    $State.UninstallPendingOperation = $null
    $State.Phase = "final_cleanup_verified"
    & $StateWriter $State
    & $BeforeMarkerRemovalAction
    & $MarkerRemovalAction
    & $AfterMarkerRemovalAction
}

function Complete-AmeBrokerUninstallTransaction {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$AfterCommittedAction = {},
        [scriptblock]$BeforeMarkerRemovalAction = {},
        [scriptblock]$MarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [scriptblock]$AfterMarkerRemovalAction = {},
        [hashtable]$ActionOverrides,
        [string]$FaultOperation = "",
        [string]$FaultBoundary = "",
        [scriptblock]$FaultAction = {}
    )

    if ([string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction may use uninstall completion"
    }
    Assert-AmeBrokerUninstallProgressFacts -State $State
    $removeApplicationBundle = Test-AmeBrokerUninstallApplicationRemoval -State $State
    $knownOperations = @(Get-AmeBrokerUninstallStepDefinitions | ForEach-Object {
        [string]$_.Property
    })
    if (([string]::IsNullOrEmpty($FaultOperation)) -ne
        ([string]::IsNullOrEmpty($FaultBoundary)) -or
        (-not [string]::IsNullOrEmpty($FaultOperation) -and
            ($FaultOperation -notin $knownOperations -or
                $FaultBoundary -notin @("BeforeMutation", "AfterMutation", "AfterCompletion")))) {
        throw "The uninstall transaction fault boundary is invalid"
    }
    $installDirectory = Get-AmeBrokerInstallDirectory
    $applicationDirectory = [string]$State.ApplicationDirectory
    $installedBinary = [string]$State.InstalledBinary
    $parentPrestate = [pscustomobject]@{
        Path = [string]$State.ParentPath
        Existed = [bool]$State.ParentExisted
        Sddl = $State.ParentSddl
    }
    $treeMutation = {
        if (-not (Test-Path -LiteralPath $installDirectory -PathType Container)) {
            $created = New-AmeBrokerProtectedInstallTree -InstallDirectory $installDirectory
            $unexpected = @($created.CreatedDirectories | Where-Object {
                $createdPath = [IO.Path]::GetFullPath([string]$_)
                -not @($State.OwnedDirectories | Where-Object {
                    $createdPath.Equals(
                        [IO.Path]::GetFullPath([string]$_),
                        [StringComparison]::OrdinalIgnoreCase
                    )
                }).Count
            })
            if ($unexpected.Count -gt 0) {
                throw "Uninstall created a directory that was absent from its write-ahead ownership"
            }
        }
    }
    $treeVerify = {
        Assert-AmeBrokerFixedInstallTree `
            -InstallDirectory $installDirectory `
            -RequireInstallDirectory | Out-Null
    }
    $treeActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallProtectedTreeReady `
        -DefaultMutationAction $treeMutation `
        -DefaultVerifyAction $treeVerify
    $treeSuperseded = {
        Test-AmeBrokerUninstallPostconditionSuperseded `
            -State $State `
            -CompletionProperty UninstallProtectedTreeReady
    }
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallProtectedTreeReady `
        -PendingPhase uninstall_tree_creation_pending `
        -CompletedPhase uninstall_tree_ready `
        -MutationAction $treeActions.MutationAction `
        -VerifyAction $treeActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallProtectedTreeReady -CurrentBoundary AfterCompletion -FaultAction $FaultAction) `
        -PostconditionSupersededAction $treeSuperseded

    $serviceMutation = {
        if (Test-AmeBrokerServiceExists) {
            Remove-AmeBrokerServiceBounded
        }
    }
    $serviceVerify = {
        if (Test-AmeBrokerServiceExists) {
            throw "The broker service remains after uninstall removal"
        }
    }
    $serviceActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallServiceRemoved `
        -DefaultMutationAction $serviceMutation `
        -DefaultVerifyAction $serviceVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallServiceRemoved `
        -PendingPhase uninstall_service_removal_pending `
        -CompletedPhase uninstall_service_removed `
        -MutationAction $serviceActions.MutationAction `
        -VerifyAction $serviceActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallServiceRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $binaryMutation = {
        Remove-AmeBrokerOwnedLeafNoFollow `
            -Path $installedBinary `
            -AllowedPaths @($installedBinary)
    }
    $binaryVerify = {
        if (Test-Path -LiteralPath $installedBinary) {
            throw "The broker binary remains after uninstall removal"
        }
    }
    $binaryActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallBinaryRemoved `
        -DefaultMutationAction $binaryMutation `
        -DefaultVerifyAction $binaryVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallBinaryRemoved `
        -PendingPhase uninstall_binary_removal_pending `
        -CompletedPhase uninstall_binary_removed `
        -MutationAction $binaryActions.MutationAction `
        -VerifyAction $binaryActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallBinaryRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $applicationMutation = {
        Remove-AmeBrokerOwnedTreeNoFollow `
            -Path $applicationDirectory `
            -ExpectedParent (Get-AmeProductParentDirectory)
    }
    $applicationVerify = {
        if (Test-Path -LiteralPath $applicationDirectory) {
            throw "The installed Application bundle remains after uninstall removal"
        }
    }
    $applicationActions = if ($removeApplicationBundle) {
        Resolve-AmeBrokerUninstallActionPair `
            -Overrides $ActionOverrides `
            -CompletionProperty UninstallApplicationRemoved `
            -DefaultMutationAction $applicationMutation `
            -DefaultVerifyAction $applicationVerify
    } else {
        [pscustomobject]@{
            MutationAction = {}
            VerifyAction = {}
        }
    }
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallApplicationRemoved `
        -PendingPhase uninstall_application_removal_pending `
        -CompletedPhase uninstall_application_removed `
        -MutationAction $applicationActions.MutationAction `
        -VerifyAction $applicationActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallApplicationRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    Set-AmeBrokerUninstallCommitted `
        -State $State `
        -StateWriter $StateWriter `
        -AfterCommittedAction $AfterCommittedAction

    $installTreeMutation = {
        if (Test-Path -LiteralPath $installDirectory -PathType Container) {
            Assert-AmeBrokerFixedInstallTree `
                -InstallDirectory $installDirectory `
                -RequireInstallDirectory | Out-Null
            if (@(Get-ChildItem -LiteralPath $installDirectory -Force).Count -ne 0) {
                throw "The broker install directory contains an unowned uninstall entry"
            }
            Remove-Item -LiteralPath $installDirectory
        }
    }
    $installTreeVerify = {
        if (Test-Path -LiteralPath $installDirectory) {
            throw "The broker install directory remains after uninstall cleanup"
        }
    }
    $installTreeActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallInstallTreeRemoved `
        -DefaultMutationAction $installTreeMutation `
        -DefaultVerifyAction $installTreeVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallInstallTreeRemoved `
        -PendingPhase uninstall_install_tree_removal_pending `
        -CompletedPhase uninstall_install_tree_removed `
        -MutationAction $installTreeActions.MutationAction `
        -VerifyAction $installTreeActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallInstallTreeRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $ownedDirectoriesMutation = {
        $failures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories @($State.OwnedDirectories | Sort-Object Length -Descending))
        if ($failures.Count -gt 0) {
            throw "Uninstall owned-directory cleanup failed: " + ($failures -join "; ")
        }
    }
    $ownedDirectoriesVerify = {
        foreach ($directory in @($State.OwnedDirectories)) {
            if (Test-Path -LiteralPath ([string]$directory)) {
                throw "A transaction-created uninstall ancestor remains"
            }
        }
    }
    $ownedDirectoriesActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallOwnedDirectoriesRemoved `
        -DefaultMutationAction $ownedDirectoriesMutation `
        -DefaultVerifyAction $ownedDirectoriesVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallOwnedDirectoriesRemoved `
        -PendingPhase uninstall_owned_directories_removal_pending `
        -CompletedPhase uninstall_owned_directories_removed `
        -MutationAction $ownedDirectoriesActions.MutationAction `
        -VerifyAction $ownedDirectoriesActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallOwnedDirectoriesRemoved -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $parentMutation = {
        Restore-AmeBrokerParentPrestate -Prestate $parentPrestate
    }
    $parentVerify = {
        Assert-AmeBrokerParentPrestateRestored -Prestate $parentPrestate
    }
    $parentActions = Resolve-AmeBrokerUninstallActionPair `
        -Overrides $ActionOverrides `
        -CompletionProperty UninstallParentRestored `
        -DefaultMutationAction $parentMutation `
        -DefaultVerifyAction $parentVerify
    Invoke-AmeBrokerUninstallWriteAheadStep `
        -State $State `
        -CompletionProperty UninstallParentRestored `
        -PendingPhase uninstall_parent_restore_pending `
        -CompletedPhase uninstall_parent_restored `
        -MutationAction $parentActions.MutationAction `
        -VerifyAction $parentActions.VerifyAction `
        -StateWriter $StateWriter `
        -AfterPendingAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary BeforeMutation -FaultAction $FaultAction) `
        -AfterMutationAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary AfterMutation -FaultAction $FaultAction) `
        -AfterCompletedAction (Get-AmeBrokerUninstallFaultAction -FaultOperation $FaultOperation -FaultBoundary $FaultBoundary -CurrentOperation UninstallParentRestored -CurrentBoundary AfterCompletion -FaultAction $FaultAction)

    $finalVerify = {
        & $serviceActions.VerifyAction
        & $binaryActions.VerifyAction
        & $applicationActions.VerifyAction
        & $installTreeActions.VerifyAction
        & $ownedDirectoriesActions.VerifyAction
        & $parentActions.VerifyAction
    }
    Remove-AmeBrokerCompletedUninstallMarker `
        -State $State `
        -FinalVerifyAction $finalVerify `
        -StateWriter $StateWriter `
        -BeforeMarkerRemovalAction $BeforeMarkerRemovalAction `
        -MarkerRemovalAction $MarkerRemovalAction `
        -AfterMarkerRemovalAction $AfterMarkerRemovalAction
}
