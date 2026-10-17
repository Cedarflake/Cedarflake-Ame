function New-AmeBrokerRollbackAction {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Action
    )

    return [pscustomobject]@{
        Name = $Name
        Action = $Action
    }
}

function Invoke-AmeBrokerRollbackActions {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Actions
    )

    $failures = [System.Collections.Generic.List[string]]::new()
    for ($index = $Actions.Count - 1; $index -ge 0; $index -= 1) {
        $rollback = $Actions[$index]
        try {
            & $rollback.Action | Out-Null
        } catch {
            $failures.Add("$($rollback.Name): $($_.Exception.Message)")
        }
    }
    return $failures.ToArray()
}

function Throw-AmeBrokerTransactionFailure {
    param(
        [Parameter(Mandatory = $true)]
        [System.Management.Automation.ErrorRecord]$OriginalError,
        [string[]]$RollbackFailures = @()
    )

    if ($RollbackFailures.Count -gt 0) {
        $message = "$($OriginalError.Exception.Message); rollback failures: " +
            ($RollbackFailures -join "; ")
        throw [InvalidOperationException]::new($message, $OriginalError.Exception)
    }
    throw $OriginalError
}

function Remove-AmeBrokerOwnedLeafNoFollow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string[]]$AllowedPaths
    )

    $full = [IO.Path]::GetFullPath($Path)
    if (-not @($AllowedPaths | Where-Object {
        $full.Equals([IO.Path]::GetFullPath($_), [StringComparison]::OrdinalIgnoreCase)
    }).Count) {
        throw "A transaction leaf is outside its fixed ownership set"
    }
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return
    }
    $item = Get-Item -LiteralPath $full -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "A transaction leaf became a reparse point"
    }
    $final = ConvertTo-AmeBrokerComparablePath -Path (
        [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($full)
    )
    if (-not $full.Equals($final, [StringComparison]::OrdinalIgnoreCase)) {
        throw "A transaction leaf resolves outside its fixed path"
    }
    Remove-Item -LiteralPath $full -Force
}

function Test-AmeBrokerBinaryPayloadFactsMatch {
    param(
        [AllowNull()]
        [psobject]$Expected,
        [Parameter(Mandatory = $true)]
        [psobject]$Actual
    )

    if ($null -eq $Expected) {
        return $false
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Expected
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Actual
    return [bool]$Expected.Exists -and [bool]$Actual.Exists -and
        [int64]$Expected.Length -eq [int64]$Actual.Length -and
        [string]$Expected.Sha256 -ceq [string]$Actual.Sha256
}

function Get-AmeBrokerTransactionServiceFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if (-not (Test-AmeBrokerServiceExists)) {
        return [pscustomobject]@{
            Exists = $false
            BinaryPath = $null
            IdentityManifest = $null
        }
    }
    $serviceKeyPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:AmeBrokerServiceName"
    $configuration = Get-ItemProperty -LiteralPath $serviceKeyPath
    $match = [regex]::Match(
        [string]$configuration.ImagePath,
        '^"([^"]+)"$',
        [Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "The interrupted broker service path is malformed"
    }
    $binaryPath = [IO.Path]::GetFullPath($match.Groups[1].Value)
    $allowed = @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if (-not @($allowed | Where-Object {
        $binaryPath.Equals(
            [IO.Path]::GetFullPath($_),
            [StringComparison]::OrdinalIgnoreCase
        )
    }).Count) {
        throw "The interrupted broker service points outside transaction-owned leaves"
    }
    $manifest = [string]$configuration.Description
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $binaryPath `
        -IdentityManifest $manifest
    $binaryFacts = Get-AmeBrokerTransactionBinaryFacts `
        -Path $binaryPath `
        -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
    if (-not [bool]$binaryFacts.Exists) {
        throw "The interrupted broker service points to a missing binary"
    }
    $isPrevious = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedPreviousBinary `
        -Actual $binaryFacts
    $isNew = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedNewBinary `
        -Actual $binaryFacts
    if (-not (Test-AmeBrokerTransactionServiceIdentity `
        -State $State -BinaryFacts $binaryFacts -Manifest $manifest)) {
        throw "The interrupted broker service points to an unknown binary identity"
    }
    return [pscustomobject]@{
        Exists = $true
        BinaryPath = $binaryPath
        IdentityManifest = $manifest
        BinaryFacts = $binaryFacts
        IsPrevious = $isPrevious
        IsNew = $isNew
    }
}

function Get-AmeBrokerTransactionLeafFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $facts = @{}
    foreach ($path in @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    )) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        $binary = Get-AmeBrokerTransactionBinaryFacts `
            -Path $path `
            -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
        if ([bool]$binary.Exists -and
            -not (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $State.ExpectedPreviousBinary `
                -Actual $binary) -and
            -not (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $State.ExpectedNewBinary `
                -Actual $binary)) {
            throw "A transaction leaf contains an unknown signed broker payload"
        }
        $facts[[IO.Path]::GetFullPath($path)] = $binary
    }
    return $facts
}

function Find-AmeBrokerTransactionLeafByPayload {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$LeafFacts,
        [AllowNull()]
        [psobject]$Expected,
        [string[]]$Preference = @()
    )

    foreach ($path in $Preference) {
        $full = [IO.Path]::GetFullPath($path)
        if ($LeafFacts.ContainsKey($full) -and
            (Test-AmeBrokerBinaryPayloadFactsMatch `
                -Expected $Expected `
                -Actual $LeafFacts[$full])) {
            return $full
        }
    }
    return $null
}

function Repair-AmeBrokerInterruptedBinaryTransaction {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if ($null -eq $State.ExpectedNewBinary) {
        if ($null -ne $State.PendingIntent -or @($State.AppliedOperations).Count -gt 0) {
            throw "A moved binary transaction lacks its expected new identity"
        }
        foreach ($path in @([string]$State.StagedBinary, [string]$State.BackupBinary)) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $path `
                -AllowedPaths @(
                    [string]$State.InstalledBinary,
                    [string]$State.StagedBinary,
                    [string]$State.BackupBinary
                )
        }
        if ([string]$State.Operation -ceq "upgrade") {
            $installedFacts = Get-AmeBrokerTransactionBinaryFacts `
                -Path ([string]$State.InstalledBinary) `
                -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
            if (-not (Test-AmeBrokerBinaryFactsMatch `
                -Expected $State.ExpectedPreviousBinary `
                -Actual $installedFacts)) {
                throw "The pre-move upgrade binary no longer matches its journaled identity"
            }
            Assert-AmeBrokerServiceConfiguration `
                -BinaryPath ([string]$State.InstalledBinary) `
                -IdentityManifest ([string]$State.PreviousIdentityManifest)
        }
        Remove-AmeBrokerTransactionMarker
        return
    }
    if ([string]::IsNullOrWhiteSpace([string]$State.NewIdentityManifest)) {
        throw "The interrupted binary transaction lacks its expected new manifest"
    }
    $serviceFacts = Get-AmeBrokerTransactionServiceFacts -State $State
    if (-not [bool]$serviceFacts.Exists) {
        throw "The interrupted binary transaction lost its fixed service"
    }
    if ($null -ne $State.PendingIntent) {
        $pendingSource = Get-AmeBrokerTransactionBinaryFacts `
            -Path ([string]$State.PendingIntent.SourcePath) `
            -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
        $pendingDestination = Get-AmeBrokerTransactionBinaryFacts `
            -Path ([string]$State.PendingIntent.DestinationPath) `
            -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
        $pendingResolution = Resolve-AmeBrokerMoveIntentFacts `
            -Intent $State.PendingIntent `
            -SourceFacts $pendingSource `
            -DestinationFacts $pendingDestination
        if ($pendingResolution -ceq "NotApplied" -and
            ([IO.Path]::GetFullPath([string]$serviceFacts.BinaryPath)).Equals(
                [IO.Path]::GetFullPath([string]$State.PendingIntent.SourcePath),
                [StringComparison]::OrdinalIgnoreCase
            )) {
            throw "The broker service still points to a pending move source"
        }
        Resolve-AmeBrokerPendingMove -State $State
        $serviceFacts = Get-AmeBrokerTransactionServiceFacts -State $State
    }
    $leafFacts = Get-AmeBrokerTransactionLeafFacts -State $State
    $installedPath = [IO.Path]::GetFullPath([string]$State.InstalledBinary)
    $stagedPath = [IO.Path]::GetFullPath([string]$State.StagedBinary)
    $backupPath = [IO.Path]::GetFullPath([string]$State.BackupBinary)
    $previousPath = Find-AmeBrokerTransactionLeafByPayload `
        -LeafFacts $leafFacts `
        -Expected $State.ExpectedPreviousBinary `
        -Preference @($installedPath, $backupPath, $stagedPath)
    $newPath = Find-AmeBrokerTransactionLeafByPayload `
        -LeafFacts $leafFacts `
        -Expected $State.ExpectedNewBinary `
        -Preference @($installedPath, $stagedPath, $backupPath)
    $shouldRollback = $null -ne $previousPath
    if (-not $shouldRollback -and $null -eq $newPath) {
        throw "The interrupted binary transaction has no recoverable broker payload"
    }
    $targetPath = if ($shouldRollback) { $previousPath } else { $newPath }
    $targetManifest = if ($shouldRollback) {
        [string]$State.PreviousIdentityManifest
    } else {
        [string]$State.NewIdentityManifest
    }
    if ([string]::IsNullOrWhiteSpace($targetManifest)) {
        throw "The interrupted binary transaction lacks the target service manifest"
    }
    if (-not $targetPath.Equals($installedPath, [StringComparison]::OrdinalIgnoreCase)) {
        $holdingPath = $null
        foreach ($candidate in @($stagedPath, $backupPath)) {
            if ($candidate.Equals($targetPath, [StringComparison]::OrdinalIgnoreCase)) {
                continue
            }
            $candidateFacts = $leafFacts[$candidate]
            if ([bool]$candidateFacts.Exists) {
                $holdingPath = $candidate
                break
            }
        }
        if ($null -eq $holdingPath) {
            $holdingPath = if (-not $stagedPath.Equals(
                $targetPath,
                [StringComparison]::OrdinalIgnoreCase
            )) { $stagedPath } else { $backupPath }
            Copy-Item -LiteralPath $targetPath -Destination $holdingPath
            Set-AmeBrokerFileAcl -Path $holdingPath -IncludeServiceSid $true
            $leafFacts[$holdingPath] = Get-AmeBrokerTransactionBinaryFacts `
                -Path $holdingPath `
                -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
        }
        $holdingFacts = $leafFacts[$holdingPath]
        $holdingManifest = if (Test-AmeBrokerBinaryPayloadFactsMatch `
            -Expected $State.ExpectedPreviousBinary `
            -Actual $holdingFacts) {
            [string]$State.PreviousIdentityManifest
        } else {
            [string]$State.NewIdentityManifest
        }
        Set-AmeBrokerServiceConfiguration `
            -BinaryPath $holdingPath `
            -IdentityManifest $holdingManifest
        $installedFacts = $leafFacts[$installedPath]
        if ([bool]$installedFacts.Exists) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $installedPath `
                -AllowedPaths @($installedPath, $stagedPath, $backupPath)
        }
        $targetFacts = Get-AmeBrokerTransactionBinaryFacts `
            -Path $targetPath `
            -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
        Invoke-AmeBrokerWriteAheadMove `
            -State $State `
            -SourcePath $targetPath `
            -DestinationPath $installedPath `
            -ExpectedSource $targetFacts `
            -ExpectedDestinationBefore (New-AmeBrokerAbsentBinaryFacts) `
            -NextPhase recovery_installed
    }
    Set-AmeBrokerServiceConfiguration `
        -BinaryPath $installedPath `
        -IdentityManifest $targetManifest
    Set-AmeBrokerBinaryAcl `
        -InstallDirectory (Get-AmeBrokerInstallDirectory) `
        -BinaryPath $installedPath
    $finalFacts = Get-AmeBrokerTransactionBinaryFacts `
        -Path $installedPath `
        -ExpectedPublisher (Get-AmeBrokerRecoveryPublisher -State $State)
    $expectedFinal = if ($shouldRollback) {
        $State.ExpectedPreviousBinary
    } else {
        $State.ExpectedNewBinary
    }
    if (-not (Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $expectedFinal `
        -Actual $finalFacts)) {
        throw "The recovered broker payload does not match its journaled identity"
    }
    foreach ($path in @($stagedPath, $backupPath)) {
        if (-not $path.Equals($installedPath, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-AmeBrokerOwnedLeafNoFollow `
                -Path $path `
                -AllowedPaths @($installedPath, $stagedPath, $backupPath)
        }
    }
    Assert-AmeBrokerServiceConfiguration `
        -BinaryPath $installedPath `
        -IdentityManifest $targetManifest
    if ([bool]$State.WasRunning) {
        Start-AmeBrokerServiceBounded
    }
    Remove-AmeBrokerTransactionMarker
}

function Repair-AmeBrokerInterruptedTransaction {
    param(
        [scriptblock]$ProcessProbe,
        [scriptblock]$StateReader = { Read-AmeBrokerTransactionState },
        [scriptblock]$UninstallStateWriter = {
            param([psobject]$Value)
            Write-AmeBrokerTransactionState -State $Value
        },
        [scriptblock]$UninstallMarkerRemovalAction = { Remove-AmeBrokerTransactionMarker },
        [hashtable]$UninstallActionOverrides
    )

    $state = & $StateReader
    if ($null -eq $state) {
        return
    }
    Assert-AmeBrokerTransactionStateFacts -State $state
    $ownerActive = if ($null -eq $ProcessProbe) {
        Test-AmeBrokerTransactionOwnerActive -State $state
    } else {
        Test-AmeBrokerTransactionOwnerActive -State $state -ProcessProbe $ProcessProbe
    }
    if ($ownerActive) {
        throw "An active broker lifecycle transaction already owns the fixed install tree"
    }
    if ([string]$state.Operation -ceq "uninstall") {
        Complete-AmeBrokerUninstallTransaction `
            -State $state `
            -StateWriter $UninstallStateWriter `
            -MarkerRemovalAction $UninstallMarkerRemovalAction `
            -ActionOverrides $UninstallActionOverrides
        return
    }
    if ([string]$state.Operation -in @("repair", "upgrade")) {
        Repair-AmeBrokerInterruptedBinaryTransaction -State $state
        return
    }
    $installDirectory = Get-AmeBrokerInstallDirectory
    Assert-AmeBrokerFixedInstallTree `
        -InstallDirectory $installDirectory | Out-Null
    $allowedLeaves = @(
        [string]$state.InstalledBinary,
        [string]$state.StagedBinary,
        [string]$state.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    $failures = [Collections.Generic.List[string]]::new()
    if ([string]$state.Phase -ceq "committed") {
        foreach ($path in @([string]$state.StagedBinary, [string]$state.BackupBinary)) {
            if ([string]::IsNullOrWhiteSpace($path)) {
                continue
            }
            try {
                Remove-AmeBrokerOwnedLeafNoFollow -Path $path -AllowedPaths $allowedLeaves
            } catch {
                $failures.Add("finalize $path`: $($_.Exception.Message)")
            }
        }
        if ($failures.Count -eq 0) {
            Remove-AmeBrokerTransactionMarker
        }
        if ($failures.Count -gt 0) {
            throw "Interrupted committed broker transaction cleanup failed: " +
                ($failures -join "; ")
        }
        return
    }
    if ([bool]$state.WasRunning -or [bool]$state.ConfigurationChanged) {
        try {
            Stop-AmeBrokerServiceBounded
        } catch {
            $failures.Add("stop interrupted broker: $($_.Exception.Message)")
        }
    }
    if ([string]$state.Operation -ceq "install") {
        try {
            if (Test-AmeBrokerServiceExists) {
                Remove-AmeBrokerServiceBounded
            }
        } catch {
            $failures.Add("remove transaction-owned service: $($_.Exception.Message)")
        }
    }
    foreach ($path in @([string]$state.StagedBinary, [string]$state.InstalledBinary)) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        if ($path -ceq [string]$state.InstalledBinary -and
            -not [bool]$state.InstalledOwned -and
            [string]$state.Operation -cne "install") {
            continue
        }
        try {
            Remove-AmeBrokerOwnedLeafNoFollow -Path $path -AllowedPaths $allowedLeaves
        } catch {
            $failures.Add("remove transaction-owned leaf $path`: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.BackupCreated) {
        try {
            $backup = [string]$state.BackupBinary
            if (-not (Test-Path -LiteralPath $backup -PathType Leaf)) {
                throw "The transaction-owned broker backup is unavailable"
            }
            $backupItem = Get-Item -LiteralPath $backup -Force
            if ($backupItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "The transaction-owned broker backup became a reparse point"
            }
            Move-Item -LiteralPath $backup -Destination ([string]$state.InstalledBinary)
            Set-AmeBrokerBinaryAcl `
                -InstallDirectory $installDirectory `
                -BinaryPath ([string]$state.InstalledBinary)
        } catch {
            $failures.Add("restore previous broker binary: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.ConfigurationChanged -and (Test-AmeBrokerServiceExists)) {
        try {
            if ([string]::IsNullOrWhiteSpace([string]$state.PreviousIdentityManifest)) {
                throw "The previous service identity manifest is unavailable"
            }
            Set-AmeBrokerServiceConfiguration `
                -BinaryPath ([string]$state.InstalledBinary) `
                -IdentityManifest ([string]$state.PreviousIdentityManifest)
        } catch {
            $failures.Add("restore previous service configuration: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.WasRunning -and (Test-AmeBrokerServiceExists)) {
        try {
            Start-AmeBrokerServiceBounded
        } catch {
            $failures.Add("restart previous broker: $($_.Exception.Message)")
        }
    }
    if ([bool]$state.ApplicationOwned) {
        try {
            Remove-AmeBrokerOwnedTreeNoFollow `
                -Path ([string]$state.ApplicationDirectory) `
                -ExpectedParent (Get-AmeProductParentDirectory)
        } catch {
            $failures.Add("remove transaction-owned Application bundle: $($_.Exception.Message)")
        }
    }
    try {
        $ownedDirectories = @($state.OwnedDirectories | Sort-Object Length -Descending)
        $directoryFailures = @(Remove-AmeCreatedEmptyDirectories `
            -CreatedDirectories $ownedDirectories)
        foreach ($failure in $directoryFailures) {
            $failures.Add([string]$failure)
        }
    } catch {
        $failures.Add("remove transaction-owned directories: $($_.Exception.Message)")
    }
    try {
        Restore-AmeBrokerParentPrestate -Prestate ([pscustomobject]@{
            Path = [string]$state.ParentPath
            Existed = [bool]$state.ParentExisted
            Sddl = $state.ParentSddl
        })
    } catch {
        $failures.Add("restore Ame parent prestate: $($_.Exception.Message)")
    }
    if ($failures.Count -eq 0) {
        Remove-AmeBrokerTransactionMarker
    }
    if ($failures.Count -gt 0) {
        throw "Interrupted broker transaction recovery failed: " + ($failures -join "; ")
    }
}
