function New-AmeBrokerAbsentBinaryFacts {
    return [pscustomobject]@{
        Exists = $false
        Length = [int64]0
        Sha256 = $null
        FileIdentity = $null
        FinalPath = $null
    }
}

function Assert-AmeBrokerExpectedBinaryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Facts
    )

    $required = @("Exists", "Length", "Sha256", "FileIdentity", "FinalPath")
    foreach ($name in $required) {
        if ($null -eq $Facts.PSObject.Properties[$name]) {
            throw "A transaction binary fact is missing $name"
        }
    }
    if ([bool]$Facts.Exists) {
        if ([int64]$Facts.Length -lt 0 -or
            [string]$Facts.Sha256 -notmatch '^[0-9A-F]{64}$' -or
            [string]$Facts.FileIdentity -notmatch '^[0-9A-F]{8}:[0-9A-F]{16}$' -or
            [string]::IsNullOrWhiteSpace([string]$Facts.FinalPath) -or
            -not [IO.Path]::IsPathRooted([string]$Facts.FinalPath)) {
            throw "A present transaction binary fact is invalid"
        }
    } elseif ([int64]$Facts.Length -ne 0 -or
        -not [string]::IsNullOrEmpty([string]$Facts.Sha256) -or
        -not [string]::IsNullOrEmpty([string]$Facts.FileIdentity) -or
        -not [string]::IsNullOrEmpty([string]$Facts.FinalPath)) {
        throw "An absent transaction binary fact carries file identity"
    }
}

function Get-AmeBrokerTransactionBinaryFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $full = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return New-AmeBrokerAbsentBinaryFacts
    }
    $item = Get-Item -LiteralPath $full -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "A transaction binary became a reparse point"
    }
    Assert-AmeBrokerPhysicalFile -ExpectedPath $full
    Assert-AmeBrokerPayload -Path $full -ExpectedPublisher $ExpectedPublisher | Out-Null
    $facts = [pscustomobject]@{
        Exists = $true
        Length = [int64]$item.Length
        Sha256 = (Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash.ToUpperInvariant()
        FileIdentity = [Ame.JournalBrokerInstaller.NativeMethods]::GetFileIdentity($full)
        FinalPath = ConvertTo-AmeBrokerComparablePath -Path (
            [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($full)
        )
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $facts
    return $facts
}

function Test-AmeBrokerBinaryFactsMatch {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Expected,
        [Parameter(Mandatory = $true)]
        [psobject]$Actual
    )

    Assert-AmeBrokerExpectedBinaryFacts -Facts $Expected
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Actual
    if ([bool]$Expected.Exists -ne [bool]$Actual.Exists) {
        return $false
    }
    if (-not [bool]$Expected.Exists) {
        return $true
    }
    return [int64]$Expected.Length -eq [int64]$Actual.Length -and
        [string]$Expected.Sha256 -ceq [string]$Actual.Sha256 -and
        [string]$Expected.FileIdentity -ceq [string]$Actual.FileIdentity
}

function Assert-AmeBrokerMoveIntentFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Intent,
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    $required = @(
        "OperationId", "SourcePath", "DestinationPath", "ExpectedSource",
        "ExpectedDestinationBefore", "NextPhase"
    )
    foreach ($name in $required) {
        if ($null -eq $Intent.PSObject.Properties[$name]) {
            throw "A transaction move intent is missing $name"
        }
    }
    if ([string]$Intent.OperationId -notmatch (
        '^' + [regex]::Escape([string]$State.TransactionId) + ':[1-9][0-9]{0,2}$'
    ) -or [string]$Intent.NextPhase -notmatch '^[a-z][a-z0-9_]{0,63}$') {
        throw "A transaction move intent identity is invalid"
    }
    $allowed = @(
        [string]$State.InstalledBinary,
        [string]$State.StagedBinary,
        [string]$State.BackupBinary
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    foreach ($path in @([string]$Intent.SourcePath, [string]$Intent.DestinationPath)) {
        if (-not @($allowed | Where-Object {
            ([IO.Path]::GetFullPath($path)).Equals(
                [IO.Path]::GetFullPath($_),
                [StringComparison]::OrdinalIgnoreCase
            )
        }).Count) {
            throw "A transaction move intent escaped its fixed leaves"
        }
    }
    if (([IO.Path]::GetFullPath([string]$Intent.SourcePath)).Equals(
        [IO.Path]::GetFullPath([string]$Intent.DestinationPath),
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "A transaction move intent source and destination are identical"
    }
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Intent.ExpectedSource
    Assert-AmeBrokerExpectedBinaryFacts -Facts $Intent.ExpectedDestinationBefore
    if (-not [bool]$Intent.ExpectedSource.Exists) {
        throw "A transaction move intent lacks an existing expected source"
    }
}

function Assert-AmeBrokerMoveJournalFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if (@($State.AppliedOperations).Count -gt 16) {
        throw "The transaction move journal exceeded its bounded operation count"
    }
    $operationIds = [Collections.Generic.HashSet[string]]::new(
        [StringComparer]::Ordinal
    )
    foreach ($intent in @($State.AppliedOperations)) {
        Assert-AmeBrokerMoveIntentFacts -Intent $intent -State $State
        if (-not $operationIds.Add([string]$intent.OperationId)) {
            throw "The transaction move journal contains a duplicate operation"
        }
    }
    if ($null -ne $State.PendingIntent) {
        Assert-AmeBrokerMoveIntentFacts -Intent $State.PendingIntent -State $State
        if (-not $operationIds.Add([string]$State.PendingIntent.OperationId)) {
            throw "The pending move was already recorded as applied"
        }
    }
    foreach ($operationId in $operationIds) {
        $sequence = [int]$operationId.Substring($operationId.LastIndexOf(':') + 1)
        if ($sequence -ge [int]$State.NextOperationId) {
            throw "The transaction move journal sequence did not advance"
        }
    }
}

function Resolve-AmeBrokerMoveIntentFacts {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Intent,
        [Parameter(Mandatory = $true)]
        [psobject]$SourceFacts,
        [Parameter(Mandatory = $true)]
        [psobject]$DestinationFacts
    )

    if ((Test-AmeBrokerBinaryFactsMatch `
        -Expected $Intent.ExpectedSource `
        -Actual $SourceFacts) -and
        (Test-AmeBrokerBinaryFactsMatch `
            -Expected $Intent.ExpectedDestinationBefore `
            -Actual $DestinationFacts)) {
        return "NotApplied"
    }
    if ((Test-AmeBrokerBinaryFactsMatch `
        -Expected (New-AmeBrokerAbsentBinaryFacts) `
        -Actual $SourceFacts) -and
        (Test-AmeBrokerBinaryFactsMatch `
            -Expected $Intent.ExpectedSource `
            -Actual $DestinationFacts)) {
        return "Applied"
    }
    throw "The pending broker move cannot be reconciled with physical file facts"
}

function Invoke-AmeBrokerWriteAheadMove {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [string]$SourcePath,
        [Parameter(Mandatory = $true)]
        [string]$DestinationPath,
        [Parameter(Mandatory = $true)]
        [psobject]$ExpectedSource,
        [Parameter(Mandatory = $true)]
        [psobject]$ExpectedDestinationBefore,
        [Parameter(Mandatory = $true)]
        [string]$NextPhase,
        [scriptblock]$StateWriter = { param([psobject]$Value) Write-AmeBrokerTransactionState -State $Value },
        [scriptblock]$FactProbe = {
            param([string]$Path, [string]$Publisher)
            Get-AmeBrokerTransactionBinaryFacts -Path $Path -ExpectedPublisher $Publisher
        },
        [scriptblock]$MoveAction = {
            param([string]$Source, [string]$Destination)
            [Ame.JournalBrokerInstaller.NativeMethods]::MoveFileNoReplaceWriteThrough(
                $Source,
                $Destination
            )
        },
        [scriptblock]$BeforePendingAction = {},
        [scriptblock]$AfterPendingAction = {},
        [scriptblock]$AfterMoveAction = {},
        [scriptblock]$AfterAppliedAction = {}
    )

    if ($null -ne $State.PendingIntent) {
        throw "A transaction cannot begin a second move while one is pending"
    }
    $intent = [pscustomobject]@{
        OperationId = "$($State.TransactionId):$($State.NextOperationId)"
        SourcePath = [IO.Path]::GetFullPath($SourcePath)
        DestinationPath = [IO.Path]::GetFullPath($DestinationPath)
        ExpectedSource = $ExpectedSource
        ExpectedDestinationBefore = $ExpectedDestinationBefore
        NextPhase = $NextPhase
    }
    Assert-AmeBrokerMoveIntentFacts -Intent $intent -State $State
    & $BeforePendingAction
    $State.PendingIntent = $intent
    $State.NextOperationId = [int]$State.NextOperationId + 1
    & $StateWriter $State
    & $AfterPendingAction
    $sourceBefore = & $FactProbe $intent.SourcePath (Get-AmeBrokerRecoveryPublisher -State $State)
    $destinationBefore = & $FactProbe `
        $intent.DestinationPath `
        (Get-AmeBrokerRecoveryPublisher -State $State)
    if ((Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $sourceBefore `
        -DestinationFacts $destinationBefore) -cne "NotApplied") {
        throw "The broker move changed before its write-ahead action"
    }
    & $MoveAction $intent.SourcePath $intent.DestinationPath
    & $AfterMoveAction
    $sourceAfter = & $FactProbe $intent.SourcePath (Get-AmeBrokerRecoveryPublisher -State $State)
    $destinationAfter = & $FactProbe `
        $intent.DestinationPath `
        (Get-AmeBrokerRecoveryPublisher -State $State)
    if ((Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $sourceAfter `
        -DestinationFacts $destinationAfter) -cne "Applied") {
        throw "The broker move did not reach its expected physical state"
    }
    $State.AppliedOperations = @($State.AppliedOperations) + @($intent)
    $State.PendingIntent = $null
    $State.Phase = $NextPhase
    & $StateWriter $State
    & $AfterAppliedAction
}

function Resolve-AmeBrokerPendingMove {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [scriptblock]$StateWriter = { param([psobject]$Value) Write-AmeBrokerTransactionState -State $Value },
        [scriptblock]$FactProbe = {
            param([string]$Path, [string]$Publisher)
            Get-AmeBrokerTransactionBinaryFacts -Path $Path -ExpectedPublisher $Publisher
        },
        [scriptblock]$MoveAction = {
            param([string]$Source, [string]$Destination)
            [Ame.JournalBrokerInstaller.NativeMethods]::MoveFileNoReplaceWriteThrough(
                $Source,
                $Destination
            )
        }
    )

    if ($null -eq $State.PendingIntent) {
        return
    }
    $intent = $State.PendingIntent
    $source = & $FactProbe $intent.SourcePath (Get-AmeBrokerRecoveryPublisher -State $State)
    $destination = & $FactProbe `
        $intent.DestinationPath `
        (Get-AmeBrokerRecoveryPublisher -State $State)
    $resolution = Resolve-AmeBrokerMoveIntentFacts `
        -Intent $intent `
        -SourceFacts $source `
        -DestinationFacts $destination
    if ($resolution -ceq "NotApplied") {
        & $MoveAction $intent.SourcePath $intent.DestinationPath
        $source = & $FactProbe $intent.SourcePath (Get-AmeBrokerRecoveryPublisher -State $State)
        $destination = & $FactProbe `
            $intent.DestinationPath `
            (Get-AmeBrokerRecoveryPublisher -State $State)
        $resolution = Resolve-AmeBrokerMoveIntentFacts `
            -Intent $intent `
            -SourceFacts $source `
            -DestinationFacts $destination
    }
    if ($resolution -cne "Applied") {
        throw "The pending broker move did not resolve to the applied state"
    }
    $State.AppliedOperations = @($State.AppliedOperations) + @($intent)
    $State.PendingIntent = $null
    $State.Phase = [string]$intent.NextPhase
    & $StateWriter $State
}
