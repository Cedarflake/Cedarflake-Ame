$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_change_driven_reliability_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_deletion_audit_budget_tests.ps1")

function Assert-Contains {
    param([Parameter(Mandatory = $true)][string]$Value, [Parameter(Mandatory = $true)][string]$Expected)
    if ($Value.IndexOf($Expected, [StringComparison]::Ordinal) -lt 0) {
        throw "Expected R2c-R guardrail output to contain: $Expected"
    }
}

function Get-TreeShape {
    param([Parameter(Mandatory = $true)][string]$Path)
    return @(
        [IO.DirectoryInfo]::new($Path).EnumerateFileSystemInfos("*", [IO.SearchOption]::AllDirectories) |
            Sort-Object FullName |
            ForEach-Object {
                $relative = $_.FullName.Substring($Path.TrimEnd('\').Length + 1)
                $kind = if (($_.Attributes -band [IO.FileAttributes]::Directory) -ne 0) {
                    "directory"
                } else {
                    "file:$($_.Length)"
                }
                "$relative|$kind"
            }
    )
}

function Invoke-R2cRGuardrailRunnerProcess {
    param(
        [Parameter(Mandatory = $true)][string]$RunnerPath,
        [Parameter(Mandatory = $true)][string[]]$RunnerArguments,
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [Parameter(Mandatory = $true)][string]$OutputRoot
    )
    $hostExecutable = (Get-Process -Id $PID).Path
    $hostArguments = @(
        "-NoProfile", "-NonInteractive", "-File", $RunnerPath
    ) + $RunnerArguments
    return Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments $hostArguments `
        -WorkingDirectory $RepositoryRoot `
        -OutputPath (Join-Path $OutputRoot ("runner-{0}.log" -f [Guid]::NewGuid().ToString("N"))) `
        -TimeoutMilliseconds 300000
}

function Assert-R2cRVerifiedPayloadRejected {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [Parameter(Mandatory = $true)][string]$OutputRoot
    )
    $hostExecutable = (Get-Process -Id $PID).Path
    try {
        $result = Invoke-AmeR2cROwnedProcess `
            -ExecutableKind "PowerShell" `
            -FileName $hostExecutable `
            -Arguments $Arguments `
            -WorkingDirectory $RepositoryRoot `
            -OutputPath (Join-Path $OutputRoot ("payload-{0}.log" -f [Guid]::NewGuid().ToString("N"))) `
            -TimeoutMilliseconds 30000
        throw (
            "R2c-R verified process boundary accepted an adversarial payload; " +
            "exit=$($result.ExitCode) timeout=$($result.TimedOut)"
        )
    } catch {
        Assert-Contains $_.Exception.Message $Expected
    }
}

$repositoryRoot = Get-AmeRepositoryRoot
$commonPath = Join-Path $PSScriptRoot "acceptance_r2c_change_driven_reliability_common.ps1"
$toolPath = Join-Path $repositoryRoot "tool"
$nullEnvironmentValue = [Management.Automation.Language.NullString]::Value
Assert-AmeR2cRManagedNoReparseAncestors -Path $repositoryRoot
Initialize-AmeR2cRBootstrapNativeTypes
$compilerVolumeFaultObserved = $false
Start-AmeR2cRCompilerFaultForGuardrail -Point "volume-post-open-pre-transfer"
try {
    try {
        Open-AmeR2cRBootstrapDirectoryChain `
            -Path $toolPath `
            -Label "compiler volume transfer fault" | Out-Null
        throw "R2c-R compiler volume transfer fault was not injected"
    } catch {
        Assert-Contains $_.Exception.Message "injected compiler ownership fault"
        $compilerVolumeFaultObserved = $true
    }
} finally {
    Stop-AmeR2cRCompilerFaultForGuardrail
}
$compilerVolumeFaultClosed = Test-AmeR2cRCompilerFaultOwnerClosedForGuardrail
Reset-AmeR2cRCompilerFaultOwnerForGuardrail
if (-not $compilerVolumeFaultObserved -or -not $compilerVolumeFaultClosed) {
    throw "R2c-R compiler volume handle leaked before ownership transfer"
}

$compilerBootstrapFaultObserved = $false
Start-AmeR2cRCompilerFaultForGuardrail `
    -Point "bootstrap-post-create-pre-initialization"
try {
    try {
        Initialize-AmeR2cRNativeTypes -RepositoryRoot $repositoryRoot
        throw "R2c-R compiler bootstrap pre-initialization fault was not injected"
    } catch {
        Assert-Contains $_.Exception.Message "injected compiler ownership fault"
        $compilerBootstrapFaultObserved = $true
    }
} finally {
    Stop-AmeR2cRCompilerFaultForGuardrail
}
$compilerBootstrapFaultClosed = Test-AmeR2cRCompilerFaultOwnerClosedForGuardrail
Reset-AmeR2cRCompilerFaultOwnerForGuardrail
if (-not $compilerBootstrapFaultObserved -or -not $compilerBootstrapFaultClosed) {
    throw "R2c-R compiler bootstrap leaked before guarded initialization"
}
if ($null -ne ("AmeR2cRNative" -as [type])) {
    throw "R2c-R compiler bootstrap fault partially initialized native types"
}

Initialize-AmeR2cRNativeTypes -RepositoryRoot $repositoryRoot
$ordinaryAuditState = $null
$ordinaryAuditSnapshot = $null
try {
    $ordinaryAuditState = Assert-AmeR2cRDeletionSafetySource `
        -SourceText $null `
        -SourcePath $commonPath `
        -Label "ordinary audited-script identity" `
        -RetainSnapshots
    if ($ordinaryAuditState.Sources.Count -ne 1 -or
        [string]$ordinaryAuditState.Sources[0].Identity -notmatch
            '^[0-9A-F]{8}:[0-9A-F]{8}:[0-9A-F]{8}$') {
        throw "R2c-R ordinary audited-script identity was not bound"
    }
    $ordinaryAuditSnapshot = $ordinaryAuditState.Sources[0].Snapshot
    Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $ordinaryAuditState
} finally {
    if ($null -ne $ordinaryAuditState) {
        Close-AmeR2cRDeletionAuditState -State $ordinaryAuditState
    }
}
if ($null -eq $ordinaryAuditSnapshot -or
    -not [bool]$ordinaryAuditSnapshot.AllHandlesClosedForGuardrail) {
    throw "R2c-R ordinary audited-script identity handles did not close"
}

$scriptIdentityAnchor = $null
$terminalJunction = $null
$terminalJunctionIdentity = $null
try {
    $scriptIdentityAnchor = New-AmeR2cRCompilerBootstrap -RepositoryRoot $repositoryRoot
    $terminalJunctionLeaf = (
        ".ame-r2c-r-audit-$([Guid]::NewGuid().ToString('N')).ps1"
    )
    $terminalJunction = Join-Path $toolPath $terminalJunctionLeaf
    New-Item `
        -ItemType Junction `
        -Path $terminalJunction `
        -Target ([string]$scriptIdentityAnchor.Path) | Out-Null
    $terminalJunctionHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $scriptIdentityAnchor.ToolHandle `
        -LeafName $terminalJunctionLeaf `
        -Create $false `
        -RequestDelete $false
    try {
        $terminalJunctionEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $terminalJunctionHandle `
            -Label "audited-script terminal junction" `
            -AllowReparse
        $terminalJunctionIdentity = [string]$terminalJunctionEvidence.Identity
    } finally { $terminalJunctionHandle.Dispose() }
    try {
        [AmeR2cRAuditedScriptSnapshot]::Open($toolPath, $terminalJunction) | Out-Null
        throw "R2c-R unexpectedly followed an audited-script terminal reparse point"
    } catch {
        Assert-Contains $_.Exception.Message "reparse point"
    }
} finally {
    if ($null -ne $terminalJunction) {
        Remove-AmeR2cRBootstrapExpectedDirectory `
            -ParentHandle $scriptIdentityAnchor.ToolHandle `
            -LeafName ([IO.Path]::GetFileName($terminalJunction)) `
            -ExpectedIdentity $terminalJunctionIdentity `
            -ExpectReparse $true
    }
    if ($null -ne $scriptIdentityAnchor) {
        Remove-AmeR2cRCompilerBootstrap -Bootstrap $scriptIdentityAnchor
    }
}

$terminalSwapState = $null
$terminalSwapReplacementStream = $null
$terminalSwapPath = Join-Path (
    $toolPath
) (".ame-r2c-r-audit-$([Guid]::NewGuid().ToString('N')).ps1")
$terminalSwapMovedPath = "$terminalSwapPath.moved"
try {
    [IO.File]::WriteAllText(
        $terminalSwapPath,
        'Write-Output "audited-terminal"',
        [Text.UTF8Encoding]::new($false)
    )
    $terminalSwapState = Assert-AmeR2cRDeletionSafetySource `
        -SourceText $null `
        -SourcePath $terminalSwapPath `
        -Label "terminal swap identity" `
        -RetainSnapshots
    $terminalSwapState.Sources[0].Snapshot.ReleaseTerminalForGuardrail()
    [IO.File]::Move($terminalSwapPath, $terminalSwapMovedPath)
    $replacementBytes = [Text.Encoding]::UTF8.GetBytes(
        'Write-Output "replacement-terminal"'
    )
    $terminalSwapReplacementStream = [IO.FileStream]::new(
        $terminalSwapPath,
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::ReadWrite,
        ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
        4096,
        [IO.FileOptions]::DeleteOnClose
    )
    $terminalSwapReplacementStream.Write(
        $replacementBytes,
        0,
        $replacementBytes.Length
    )
    $terminalSwapReplacementStream.Flush($true)
    try {
        Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $terminalSwapState
        throw "R2c-R unexpectedly accepted an audited-script terminal swap"
    } catch {
        Assert-Contains $_.Exception.Message "terminal handle is not held"
    }
} finally {
    if ($null -ne $terminalSwapReplacementStream) {
        $terminalSwapReplacementStream.Dispose()
    }
    if ($null -ne $terminalSwapState) {
        Close-AmeR2cRDeletionAuditState -State $terminalSwapState
    }
    if ([IO.File]::Exists($terminalSwapMovedPath)) {
        [IO.File]::Move($terminalSwapMovedPath, $terminalSwapPath)
    }
    if ([IO.File]::Exists($terminalSwapPath)) {
        $terminalCleanupStream = [IO.FileStream]::new(
            $terminalSwapPath,
            [IO.FileMode]::Open,
            [IO.FileAccess]::Read,
            ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
            4096,
            [IO.FileOptions]::DeleteOnClose
        )
        $terminalCleanupStream.Dispose()
    }
}

$parentSwapAnchor = $null
$parentSwapState = $null
$parentSwapOriginalIdentity = $null
$parentSwapReplacementIdentity = $null
$parentSwapLeaf = ".ame-r2c-r-bootstrap-$([Guid]::NewGuid().ToString('N'))"
$parentSwapMovedLeaf = ".ame-r2c-r-bootstrap-$([Guid]::NewGuid().ToString('N'))"
$parentSwapPath = Join-Path $toolPath $parentSwapLeaf
$parentSwapMovedPath = Join-Path $toolPath $parentSwapMovedLeaf
$parentSwapSourceLeaf = ".ame-r2c-r-audit-$([Guid]::NewGuid().ToString('N')).ps1"
try {
    $parentSwapAnchor = New-AmeR2cRCompilerBootstrap -RepositoryRoot $repositoryRoot
    $parentSwapHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $parentSwapAnchor.ToolHandle `
        -LeafName $parentSwapLeaf `
        -Create $true `
        -RequestDelete $false
    try {
        $parentSwapOriginalIdentity = [string](
            Get-AmeR2cRBootstrapHandleEvidence `
                -Handle $parentSwapHandle `
                -Label "audited-script parent fixture"
        ).Identity
    } finally { $parentSwapHandle.Dispose() }
    $parentSwapSource = Join-Path $parentSwapPath $parentSwapSourceLeaf
    [IO.File]::WriteAllText(
        $parentSwapSource,
        'Write-Output "audited-parent"',
        [Text.UTF8Encoding]::new($false)
    )
    $parentSwapState = Assert-AmeR2cRDeletionSafetySource `
        -SourceText $null `
        -SourcePath $parentSwapSource `
        -Label "parent swap identity" `
        -RetainSnapshots
    $parentSwapState.Sources[0].Snapshot.ReleaseAllHandlesForGuardrail()
    [IO.Directory]::Move($parentSwapPath, $parentSwapMovedPath)
    $parentSwapReplacementHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $parentSwapAnchor.ToolHandle `
        -LeafName $parentSwapLeaf `
        -Create $true `
        -RequestDelete $false
    try {
        $parentSwapReplacementIdentity = [string](
            Get-AmeR2cRBootstrapHandleEvidence `
                -Handle $parentSwapReplacementHandle `
                -Label "audited-script replacement parent"
        ).Identity
    } finally { $parentSwapReplacementHandle.Dispose() }
    [IO.File]::WriteAllText(
        (Join-Path $parentSwapPath $parentSwapSourceLeaf),
        'Write-Output "replacement-parent"',
        [Text.UTF8Encoding]::new($false)
    )
    try {
        Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $parentSwapState
        throw "R2c-R unexpectedly accepted an audited-script parent swap"
    } catch {
        Assert-Contains $_.Exception.Message "parent chain is not held"
    }
} finally {
    if ($null -ne $parentSwapState) {
        Close-AmeR2cRDeletionAuditState -State $parentSwapState
    }
    foreach ($source in @(
        (Join-Path $parentSwapPath $parentSwapSourceLeaf),
        (Join-Path $parentSwapMovedPath $parentSwapSourceLeaf)
    )) {
        if ([IO.File]::Exists($source)) {
            $parentCleanupStream = [IO.FileStream]::new(
                $source,
                [IO.FileMode]::Open,
                [IO.FileAccess]::Read,
                ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
                4096,
                [IO.FileOptions]::DeleteOnClose
            )
            $parentCleanupStream.Dispose()
        }
    }
    if ($null -ne $parentSwapReplacementIdentity -and
        [IO.Directory]::Exists($parentSwapPath)) {
        Remove-AmeR2cRBootstrapExpectedDirectory `
            -ParentHandle $parentSwapAnchor.ToolHandle `
            -LeafName $parentSwapLeaf `
            -ExpectedIdentity $parentSwapReplacementIdentity `
            -ExpectReparse $false
    }
    if ($null -ne $parentSwapOriginalIdentity -and
        [IO.Directory]::Exists($parentSwapMovedPath)) {
        Remove-AmeR2cRBootstrapExpectedDirectory `
            -ParentHandle $parentSwapAnchor.ToolHandle `
            -LeafName $parentSwapMovedLeaf `
            -ExpectedIdentity $parentSwapOriginalIdentity `
            -ExpectReparse $false
    }
    if ($null -ne $parentSwapAnchor) {
        Remove-AmeR2cRCompilerBootstrap -Bootstrap $parentSwapAnchor
    }
}

$auditFaultObserved = $false
[AmeR2cRAuditedScriptSnapshot]::BeginGuardrailFault("terminal-post-open")
try {
    try {
        [AmeR2cRAuditedScriptSnapshot]::Open($toolPath, $commonPath) | Out-Null
        throw "R2c-R audited-script terminal fault was not injected"
    } catch {
        Assert-Contains $_.Exception.Message "injected audited-script handle fault"
        $auditFaultObserved = $true
    }
} finally {
    [AmeR2cRAuditedScriptSnapshot]::EndGuardrailFault()
}
$auditFaultHandleClosed = [AmeR2cRAuditedScriptSnapshot]::GuardrailFaultHandleIsClosed
[AmeR2cRAuditedScriptSnapshot]::ResetGuardrailFaultHandle()
if (-not $auditFaultObserved -or -not $auditFaultHandleClosed) {
    throw "R2c-R audited-script exceptional path leaked a native handle"
}

[AmeR2cRAuditedScriptSnapshot]::ResetGuardrailOwnershipCounts()
$duplicateIdentityState = New-AmeR2cRDeletionAuditState
try {
    $firstIdentityRecord = Add-AmeR2cRDeletionAuditSource `
        -State $duplicateIdentityState `
        -SourceText $null `
        -Label "ordinary duplicate identity first" `
        -SourcePath $commonPath `
        -IsRoot $true `
        -Depth 0
    $secondIdentityRecord = Add-AmeR2cRDeletionAuditSource `
        -State $duplicateIdentityState `
        -SourceText $null `
        -Label "ordinary duplicate identity second" `
        -SourcePath $commonPath `
        -IsRoot $false `
        -Depth 0
    if (-not [object]::ReferenceEquals($firstIdentityRecord, $secondIdentityRecord) -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailOpenCount -ne 2 -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailDisposeCount -ne 1 -or
        $duplicateIdentityState.HeldStreams.Count -ne 1 -or
        $duplicateIdentityState.Sources.Count -ne 1 -or
        $duplicateIdentityState.SourceByIdentity.Count -ne 1) {
        throw "R2c-R ordinary duplicate identity was not opened, identity-deduplicated, and singly owned"
    }
} finally { Close-AmeR2cRDeletionAuditState -State $duplicateIdentityState }
if ([AmeR2cRAuditedScriptSnapshot]::GuardrailDisposeCount -ne 2) {
    throw "R2c-R ordinary duplicate identity did not close both snapshot owners exactly once"
}

[AmeR2cRAuditedScriptSnapshot]::ResetGuardrailOwnershipCounts()
$caseDisabledState = New-AmeR2cRDeletionAuditState
$caseDisabledLeaf = (
    [IO.Path]::GetFileNameWithoutExtension($commonPath).ToUpperInvariant() + ".ps1"
)
$caseDisabledAlias = Join-Path ([IO.Path]::GetDirectoryName($commonPath)) $caseDisabledLeaf
try {
    $caseDisabledFirst = Add-AmeR2cRDeletionAuditSource `
        -State $caseDisabledState `
        -SourceText $null `
        -Label "case-disabled original spelling" `
        -SourcePath $commonPath `
        -IsRoot $true `
        -Depth 0
    $caseDisabledSecond = Add-AmeR2cRDeletionAuditSource `
        -State $caseDisabledState `
        -SourceText $null `
        -Label "case-disabled alternate spelling" `
        -SourcePath $caseDisabledAlias `
        -IsRoot $false `
        -Depth 0
    if (-not [object]::ReferenceEquals($caseDisabledFirst, $caseDisabledSecond) -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailOpenCount -ne 2 -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailDisposeCount -ne 1 -or
        $caseDisabledState.SourceByPath.Count -ne 2 -or
        -not [object]::ReferenceEquals(
            $caseDisabledState.SourceByPath.Comparer,
            [StringComparer]::Ordinal
        )) {
        throw "R2c-R case-disabled alias did not defer deduplication until held file identity"
    }
} finally { Close-AmeR2cRDeletionAuditState -State $caseDisabledState }

[AmeR2cRAuditedScriptSnapshot]::ResetGuardrailOwnershipCounts()
$exceptionalIdentityState = New-AmeR2cRDeletionAuditState
try {
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $exceptionalIdentityState `
            -SourceText "not the held file text" `
            -Label "exceptional duplicate ownership" `
            -SourcePath $commonPath `
            -IsRoot $true `
            -Depth 0 | Out-Null
        throw "R2c-R exceptional identity fixture accepted mismatched supplied text"
    } catch {
        Assert-Contains $_.Exception.Message "source text changed before identity binding"
    }
    if ($exceptionalIdentityState.HeldStreams.Count -ne 0 -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailOpenCount -ne 1 -or
        [AmeR2cRAuditedScriptSnapshot]::GuardrailDisposeCount -ne 1) {
        throw "R2c-R exceptional identity path did not close its untransferred snapshot"
    }
} finally { Close-AmeR2cRDeletionAuditState -State $exceptionalIdentityState }

$caseSensitiveBootstrap = $null
$caseSensitiveEnabled = $false
$caseSensitiveSkip = $null
$caseSensitiveUpperPath = $null
$caseSensitiveLowerPath = $null
try {
    $caseSensitiveBootstrap = New-AmeR2cRCompilerBootstrap -RepositoryRoot $repositoryRoot
    try {
        Set-AmeR2cRBootstrapCaseSensitivityForGuardrail `
            -Bootstrap $caseSensitiveBootstrap `
            -Enabled $true
        $caseSensitiveEnabled = $true
    } catch {
        if ($_.Exception.Message -match 'Win32=(5|50)(?:\D|$)') {
            $caseSensitiveSkip = $_.Exception.Message
        } else {
            throw
        }
    }
    if ($caseSensitiveEnabled) {
        $caseSensitiveUpperPath = Join-Path $caseSensitiveBootstrap.Path "Safe.ps1"
        $caseSensitiveLowerPath = Join-Path $caseSensitiveBootstrap.Path "safe.ps1"
        [IO.File]::WriteAllText(
            $caseSensitiveUpperPath,
            '. (Join-Path $PSScriptRoot "safe.ps1")',
            [Text.UTF8Encoding]::new($false)
        )
        [IO.File]::WriteAllText(
            $caseSensitiveLowerPath,
            'if ($false) { [System.IO.File]::Delete("owned") }',
            [Text.UTF8Encoding]::new($false)
        )
        try {
            Assert-AmeR2cRDeletionSafetySource `
                -SourceText $null `
                -SourcePath $caseSensitiveUpperPath `
                -Label "case-sensitive source identity"
            throw "R2c-R case-sensitive source identity collapsed two independent files"
        } catch {
            Assert-Contains $_.Exception.Message "path-addressed Delete"
        }
        Write-Output "AME_R2C_R_CASE_SENSITIVE_SOURCE status=passed"
    } else {
        Write-Output (
            "AME_R2C_R_CASE_SENSITIVE_SOURCE status=skipped reason=" +
            $caseSensitiveSkip.Replace([Environment]::NewLine, " ")
        )
    }
} finally {
    foreach ($fixturePath in @($caseSensitiveLowerPath, $caseSensitiveUpperPath)) {
        if (-not [string]::IsNullOrEmpty($fixturePath) -and [IO.File]::Exists($fixturePath)) {
            $caseSensitiveCleanup = [IO.FileStream]::new(
                $fixturePath,
                [IO.FileMode]::Open,
                [IO.FileAccess]::Read,
                ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
                4096,
                [IO.FileOptions]::DeleteOnClose
            )
            $caseSensitiveCleanup.Dispose()
        }
    }
    if ($caseSensitiveEnabled) {
        Set-AmeR2cRBootstrapCaseSensitivityForGuardrail `
            -Bootstrap $caseSensitiveBootstrap `
            -Enabled $false
    }
    if ($null -ne $caseSensitiveBootstrap) {
        Remove-AmeR2cRCompilerBootstrap -Bootstrap $caseSensitiveBootstrap
    }
}

foreach ($budgetKey in @(
    "source-count",
    "dot-source-depth",
    "per-source-bytes",
    "total-bytes",
    "ast-nodes",
    "function-count",
    "scope-count",
    "queue-count"
)) {
    $below = Add-AmeR2cRDeletionAuditBudgetValue `
        -Key $budgetKey `
        -Current 0 `
        -Delta 15 `
        -Limit 16
    $at = Add-AmeR2cRDeletionAuditBudgetValue `
        -Key $budgetKey `
        -Current 0 `
        -Delta 16 `
        -Limit 16
    if ($below -ne 15 -or $at -ne 16) {
        throw "R2c-R budget '$budgetKey' rejected limit-1 or limit"
    }
    try {
        Add-AmeR2cRDeletionAuditBudgetValue `
            -Key $budgetKey `
            -Current 16 `
            -Delta 1 `
            -Limit 16 | Out-Null
        throw "R2c-R budget '$budgetKey' accepted limit+1"
    } catch {
        Assert-Contains $_.Exception.Message "budget=$budgetKey limit=16 actual=17"
    }
}
try {
    Add-AmeR2cRDeletionAuditBudgetValue `
        -Key "total-bytes" `
        -Current ([uint64]::MaxValue) `
        -Delta 1 `
        -Limit ([uint64]::MaxValue) | Out-Null
    throw "R2c-R budget arithmetic accepted uint64 overflow"
} catch {
    Assert-Contains $_.Exception.Message "budget=total-bytes"
    Assert-Contains $_.Exception.Message "actual=overflow"
}

foreach ($length in @(15, 16, 17)) {
    $budgetState = New-AmeR2cRDeletionAuditState `
        -BudgetOverridesForGuardrail @{ "per-source-bytes" = [uint64]16 }
    try {
        $source = "#" + ("x" * ($length - 1))
        if ($length -le 16) {
            Add-AmeR2cRDeletionAuditSource `
                -State $budgetState `
                -SourceText $source `
                -Label "per-source-$length" `
                -SourcePath $null `
                -IsRoot $true `
                -Depth 0 | Out-Null
        } else {
            try {
                Add-AmeR2cRDeletionAuditSource `
                    -State $budgetState `
                    -SourceText $source `
                    -Label "per-source-$length" `
                    -SourcePath $null `
                    -IsRoot $true `
                    -Depth 0 | Out-Null
                throw "R2c-R per-source byte budget accepted limit+1"
            } catch {
                Assert-Contains $_.Exception.Message "budget=per-source-bytes"
            }
        }
    } finally { Close-AmeR2cRDeletionAuditState -State $budgetState }
}

$sourceCountState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{ "source-count" = [uint64]2 }
try {
    foreach ($index in 1..2) {
        Add-AmeR2cRDeletionAuditSource `
            -State $sourceCountState `
            -SourceText "#$index" `
            -Label "source-count-$index" `
            -SourcePath $null `
            -IsRoot ($index -eq 1) `
            -Depth 0 | Out-Null
    }
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $sourceCountState `
            -SourceText "#3" `
            -Label "source-count-3" `
            -SourcePath $null `
            -IsRoot $false `
            -Depth 0 | Out-Null
        throw "R2c-R source-count budget accepted a third source"
    } catch { Assert-Contains $_.Exception.Message "budget=source-count" }
} finally { Close-AmeR2cRDeletionAuditState -State $sourceCountState }

$depthState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{ "dot-source-depth" = [uint64]1 }
try {
    foreach ($depth in @([uint64]0, [uint64]1)) {
        Add-AmeR2cRDeletionAuditSource `
            -State $depthState `
            -SourceText "#depth-$depth" `
            -Label "depth-$depth" `
            -SourcePath $null `
            -IsRoot ($depth -eq 0) `
            -Depth $depth | Out-Null
    }
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $depthState `
            -SourceText "#depth-2" `
            -Label "depth-2" `
            -SourcePath $null `
            -IsRoot $false `
            -Depth 2 | Out-Null
        throw "R2c-R dot-source depth budget accepted limit+1"
    } catch { Assert-Contains $_.Exception.Message "budget=dot-source-depth" }
} finally { Close-AmeR2cRDeletionAuditState -State $depthState }

$totalState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{
        "per-source-bytes" = [uint64]4
        "total-bytes" = [uint64]4
    }
try {
    foreach ($label in @("first", "second")) {
        Add-AmeR2cRDeletionAuditSource `
            -State $totalState `
            -SourceText "#x" `
            -Label $label `
            -SourcePath $null `
            -IsRoot ($label -ceq "first") `
            -Depth 0 | Out-Null
    }
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $totalState `
            -SourceText "#" `
            -Label "third" `
            -SourcePath $null `
            -IsRoot $false `
            -Depth 0 | Out-Null
        throw "R2c-R total byte budget accepted limit+1"
    } catch { Assert-Contains $_.Exception.Message "budget=total-bytes" }
} finally { Close-AmeR2cRDeletionAuditState -State $totalState }

Test-AmeR2cRDeletionAuditNodeBudgets

$functionState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{ "function-count" = [uint64]1 }
try {
    Add-AmeR2cRDeletionAuditSource `
        -State $functionState `
        -SourceText "function One {}" `
        -Label "function-one" `
        -SourcePath $null `
        -IsRoot $true `
        -Depth 0 | Out-Null
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $functionState `
            -SourceText "function Two {}" `
            -Label "function-two" `
            -SourcePath $null `
            -IsRoot $false `
            -Depth 0 | Out-Null
        throw "R2c-R function-count budget accepted limit+1"
    } catch { Assert-Contains $_.Exception.Message "budget=function-count" }
} finally { Close-AmeR2cRDeletionAuditState -State $functionState }

$scopeState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{
        "scope-count" = [uint64]1
        "queue-count" = [uint64]2
    }
try {
    $scopeRecord = Add-AmeR2cRDeletionAuditSource `
        -State $scopeState `
        -SourceText "function One {}" `
        -Label "scope-budget" `
        -SourcePath $null `
        -IsRoot $true `
        -Depth 0
    Add-AmeR2cRDeletionAuditScope `
        -State $scopeState `
        -Record $scopeRecord `
        -Function $null
    $scopeFunction = @($scopeRecord.Ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst]
    }, $true))[0]
    try {
        Add-AmeR2cRDeletionAuditScope `
            -State $scopeState `
            -Record $scopeRecord `
            -Function $scopeFunction
        throw "R2c-R scope-count budget accepted limit+1"
    } catch { Assert-Contains $_.Exception.Message "budget=scope-count" }
} finally { Close-AmeR2cRDeletionAuditState -State $scopeState }

$queueState = New-AmeR2cRDeletionAuditState `
    -BudgetOverridesForGuardrail @{
        "scope-count" = [uint64]2
        "queue-count" = [uint64]1
    }
try {
    $queueRecord = Add-AmeR2cRDeletionAuditSource `
        -State $queueState `
        -SourceText "function One {}" `
        -Label "queue-budget" `
        -SourcePath $null `
        -IsRoot $true `
        -Depth 0
    Add-AmeR2cRDeletionAuditScope `
        -State $queueState `
        -Record $queueRecord `
        -Function $null
    $queueFunction = @($queueRecord.Ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst]
    }, $true))[0]
    try {
        Add-AmeR2cRDeletionAuditScope `
            -State $queueState `
            -Record $queueRecord `
            -Function $queueFunction
        throw "R2c-R queue-count budget accepted limit+1"
    } catch { Assert-Contains $_.Exception.Message "budget=queue-count" }
} finally { Close-AmeR2cRDeletionAuditState -State $queueState }

$bootstrapAttackFixture = New-AmeR2cRCompilerBootstrap -RepositoryRoot $repositoryRoot
$bootstrapAttackRoot = [string]$bootstrapAttackFixture.Path
$hostileTemp = $bootstrapAttackRoot
$hostileTmp = $bootstrapAttackRoot
$junctionTarget = $bootstrapAttackRoot
$bootstrapSentinelStream = [IO.FileStream]::new(
    (Join-Path $bootstrapAttackRoot "sentinel.bin"),
    [IO.FileMode]::CreateNew,
    [IO.FileAccess]::ReadWrite,
    ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
    4096,
    [IO.FileOptions]::DeleteOnClose
)
$bootstrapSentinelStream.Write([byte[]](1, 2, 3, 4), 0, 4)
$bootstrapSentinelStream.Flush($true)
$hostileTempBefore = @(Get-TreeShape -Path $hostileTemp)
$hostileTmpBefore = @(Get-TreeShape -Path $hostileTmp)
$junctionTargetBefore = @(Get-TreeShape -Path $junctionTarget)
$bootstrapBefore = @(
    [IO.DirectoryInfo]::new($toolPath).EnumerateFileSystemInfos() |
        Where-Object { $_.Name -like ".ame-r2c-r-bootstrap-*" } |
        Select-Object -ExpandProperty Name |
        Sort-Object
)
$freshPreviousTemp = [Environment]::GetEnvironmentVariable("TEMP", "Process")
$freshPreviousTmp = [Environment]::GetEnvironmentVariable("TMP", "Process")
$activeBootstrap = $null
$precreatedJunction = $null
$precreatedJunctionIdentity = $null
try {
    [Environment]::SetEnvironmentVariable("TEMP", $hostileTemp, "Process")
    [Environment]::SetEnvironmentVariable("TMP", $hostileTmp, "Process")
    $escapedCommon = $commonPath.Replace("'", "''")
    $escapedRepository = $repositoryRoot.Replace("'", "''")
    $childCommand = @"
`$ErrorActionPreference = "Stop"
if (`$null -ne ("AmeR2cRNative" -as [type])) { throw "R2c-R fresh child unexpectedly preloaded native types" }
. '$escapedCommon'
if (`$null -ne ("AmeR2cRNative" -as [type])) { throw "R2c-R common dot-source performed dynamic compilation" }
Initialize-AmeR2cRNativeTypes -RepositoryRoot '$escapedRepository'
if (`$null -eq ("AmeR2cRNative" -as [type]) -or `$null -eq ("AmeR2cRDisposableRoot" -as [type])) { throw "R2c-R trusted bootstrap did not load native types" }
Write-Output "AME_R2C_R_FRESH_BOOTSTRAP status=passed"
"@
    $encodedChildCommand = [Convert]::ToBase64String(
        [Text.Encoding]::Unicode.GetBytes($childCommand)
    )
    $hostExecutable = (Get-Process -Id $PID).Path
    $freshResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments @(
            "-NoProfile", "-NonInteractive", "-EncodedCommand", $encodedChildCommand
        ) `
        -WorkingDirectory $repositoryRoot `
        -OutputPath (Join-Path $bootstrapAttackRoot "fresh-child.log") `
        -TimeoutMilliseconds 300000
    $freshOutput = @($freshResult.Lines)
    if ($freshResult.TimedOut -or $freshResult.ExitCode -ne 0) {
        throw "R2c-R fresh hostile-TEMP bootstrap failed: $($freshOutput -join [Environment]::NewLine)"
    }
    Assert-Contains ($freshOutput | Out-String) "AME_R2C_R_FRESH_BOOTSTRAP status=passed"

    $failureChildCommand = @"
`$ErrorActionPreference = "Stop"
. '$escapedCommon'
try {
    Initialize-AmeR2cRNativeTypes -RepositoryRoot '$escapedRepository' -ForceCompilerFailure
    throw "R2c-R forced compiler failure unexpectedly succeeded"
} catch {
    if (`$_.Exception.Message -notmatch 'compil|编译') { throw }
}
if (`$null -ne ("AmeR2cRNative" -as [type])) { throw "R2c-R compiler failure partially loaded native types" }
Write-Output "AME_R2C_R_COMPILER_FAILURE status=passed"
"@
    $encodedFailureCommand = [Convert]::ToBase64String(
        [Text.Encoding]::Unicode.GetBytes($failureChildCommand)
    )
    $failureResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments @(
            "-NoProfile", "-NonInteractive", "-EncodedCommand", $encodedFailureCommand
        ) `
        -WorkingDirectory $repositoryRoot `
        -OutputPath (Join-Path $bootstrapAttackRoot "compiler-failure-child.log") `
        -TimeoutMilliseconds 300000
    $failureOutput = @($failureResult.Lines)
    if ($failureResult.TimedOut -or $failureResult.ExitCode -ne 0) {
        throw "R2c-R compiler-failure guard failed: $($failureOutput -join [Environment]::NewLine)"
    }
    Assert-Contains ($failureOutput | Out-String) "AME_R2C_R_COMPILER_FAILURE status=passed"

    Initialize-AmeR2cRBootstrapNativeTypes
    $activeBootstrap = New-AmeR2cRCompilerBootstrap -RepositoryRoot $repositoryRoot
    $movedBootstrap = "$($activeBootstrap.Path)-moved"
    $escapedBootstrap = ([string]$activeBootstrap.Path).Replace("'", "''")
    $escapedMovedBootstrap = $movedBootstrap.Replace("'", "''")
    $replacementCommand = @"
`$ErrorActionPreference = "Stop"
try {
    Move-Item -LiteralPath '$escapedBootstrap' -Destination '$escapedMovedBootstrap'
    Write-Output "AME_R2C_R_BOOTSTRAP_REPLACEMENT status=moved"
} catch {
    Write-Output "AME_R2C_R_BOOTSTRAP_REPLACEMENT status=blocked"
}
"@
    $encodedReplacement = [Convert]::ToBase64String(
        [Text.Encoding]::Unicode.GetBytes($replacementCommand)
    )
    $replacementResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments @(
            "-NoProfile", "-NonInteractive", "-EncodedCommand", $encodedReplacement
        ) `
        -WorkingDirectory $repositoryRoot `
        -OutputPath (Join-Path $bootstrapAttackRoot "replacement-child.log") `
        -TimeoutMilliseconds 300000
    $replacementOutput = @($replacementResult.Lines)
    if ($replacementResult.TimedOut -or $replacementResult.ExitCode -ne 0 -or
        @($replacementOutput | Where-Object {
            $_ -ceq "AME_R2C_R_BOOTSTRAP_REPLACEMENT status=blocked"
        }).Count -ne 1) {
        throw "R2c-R active bootstrap replacement was not blocked"
    }
    Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $activeBootstrap
    try {
        Remove-AmeR2cRCompilerBootstrap -Bootstrap $activeBootstrap
    } catch { throw "R2c-R active-bootstrap cleanup failed: $($_.Exception.Message)" }
    $activeBootstrap = $null
    if ([IO.Directory]::Exists($movedBootstrap)) {
        throw "R2c-R active bootstrap replacement left a moved directory"
    }

    $junctionNonce = [Guid]::NewGuid().ToString("N")
    $junctionLeaf = ".ame-r2c-r-bootstrap-$junctionNonce"
    $precreatedJunction = Join-Path $toolPath $junctionLeaf
    New-Item -ItemType Junction -Path $precreatedJunction -Target $junctionTarget | Out-Null
    $precreatedJunctionHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $bootstrapAttackFixture.ToolHandle `
        -LeafName $junctionLeaf `
        -Create $false `
        -RequestDelete $false
    try {
        $precreatedJunctionEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $precreatedJunctionHandle `
            -Label "pre-created compiler junction" `
            -AllowReparse
        if (-not [bool]$precreatedJunctionEvidence.IsReparse) {
            throw "R2c-R pre-created compiler junction lost its reparse identity"
        }
        $precreatedJunctionIdentity = [string]$precreatedJunctionEvidence.Identity
    } finally { $precreatedJunctionHandle.Dispose() }
    try {
        New-AmeR2cRCompilerBootstrap `
            -RepositoryRoot $repositoryRoot `
            -LeafName $junctionLeaf | Out-Null
        throw "R2c-R unexpectedly accepted a pre-created compiler junction"
    } catch {
        Assert-Contains $_.Exception.Message "could not create relative directory"
    }
    $junctionTargetAfter = @(Get-TreeShape -Path $junctionTarget)
    if (($junctionTargetAfter -join "`n") -cne ($junctionTargetBefore -join "`n")) {
        throw (
            "R2c-R pre-created compiler junction changed its sentinel target; before=" +
            "[$($junctionTargetBefore -join ';')] after=[$($junctionTargetAfter -join ';')]"
        )
    }
    Remove-AmeR2cRBootstrapExpectedDirectory `
        -ParentHandle $bootstrapAttackFixture.ToolHandle `
        -LeafName $junctionLeaf `
        -ExpectedIdentity $precreatedJunctionIdentity `
        -ExpectReparse $true
    $precreatedJunction = $null
    $precreatedJunctionIdentity = $null
    if (((Get-TreeShape -Path $hostileTemp) -join "`n") -cne ($hostileTempBefore -join "`n")) {
        throw "R2c-R fresh bootstrap wrote into the hostile TEMP sentinel"
    }
    if (((Get-TreeShape -Path $hostileTmp) -join "`n") -cne ($hostileTmpBefore -join "`n")) {
        throw "R2c-R fresh bootstrap wrote into the hostile TMP sentinel"
    }
    $bootstrapAfter = @(
        [IO.DirectoryInfo]::new($toolPath).EnumerateFileSystemInfos() |
            Where-Object { $_.Name -like ".ame-r2c-r-bootstrap-*" } |
            Select-Object -ExpandProperty Name |
            Sort-Object
    )
    if (($bootstrapAfter -join "`n") -cne ($bootstrapBefore -join "`n")) {
        throw "R2c-R fresh bootstrap left a repository-owned temporary directory"
    }
} finally {
    if ($null -eq $freshPreviousTemp) {
        [Environment]::SetEnvironmentVariable(
            "TEMP",
            $nullEnvironmentValue,
            [EnvironmentVariableTarget]::Process
        )
    } else {
        [Environment]::SetEnvironmentVariable(
            "TEMP",
            $freshPreviousTemp,
            [EnvironmentVariableTarget]::Process
        )
    }
    if ($null -eq $freshPreviousTmp) {
        [Environment]::SetEnvironmentVariable(
            "TMP",
            $nullEnvironmentValue,
            [EnvironmentVariableTarget]::Process
        )
    } else {
        [Environment]::SetEnvironmentVariable(
            "TMP",
            $freshPreviousTmp,
            [EnvironmentVariableTarget]::Process
        )
    }
    if ($null -ne $activeBootstrap) {
        try {
            Remove-AmeR2cRCompilerBootstrap -Bootstrap $activeBootstrap
        } catch { throw "R2c-R retained active-bootstrap cleanup failed: $($_.Exception.Message)" }
    }
    if ($null -ne $precreatedJunction) {
        if ([string]::IsNullOrEmpty($precreatedJunctionIdentity)) {
            throw "R2c-R refused to clean a compiler junction without its captured identity"
        }
        Remove-AmeR2cRBootstrapExpectedDirectory `
            -ParentHandle $bootstrapAttackFixture.ToolHandle `
            -LeafName ([IO.Path]::GetFileName($precreatedJunction)) `
            -ExpectedIdentity $precreatedJunctionIdentity `
            -ExpectReparse $true
    }
    if ($null -ne $bootstrapSentinelStream) { $bootstrapSentinelStream.Dispose() }
    if ($null -ne $bootstrapAttackFixture) {
        try {
            Remove-AmeR2cRCompilerBootstrap -Bootstrap $bootstrapAttackFixture
        } catch { throw "R2c-R bootstrap-attack cleanup failed: $($_.Exception.Message)" }
    }
}

$bindFaultObserved = $false
try {
    [AmeR2cRDisposableRoot]::ProbeBindDirectoryChainPostOpenFaultForGuardrail($toolPath)
} catch {
    Assert-Contains $_.Exception.Message "injected post-open handle fault"
    $bindFaultObserved = $true
}
$bindFaultHandleClosed = [AmeR2cRDisposableRoot]::GuardrailFaultHandleIsClosed
[AmeR2cRDisposableRoot]::ReleaseGuardrailFaultHandleForCleanup()
if (-not $bindFaultObserved -or -not $bindFaultHandleClosed) {
    throw "R2c-R BindDirectoryChain did not close its injected post-open handle"
}

$deleteFaultFixture = $null
$deleteFaultOriginalPath = $null
$deleteFaultMovedPath = $null
$deleteFaultMovedLeaf = $null
try {
    $deleteFaultFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $deleteFaultOriginalPath = [string]$deleteFaultFixture.Path
    $deleteFaultMovedLeaf = (
        "$($deleteFaultFixture.RootName)-moved-$([Guid]::NewGuid().ToString('N'))"
    )
    $deleteFaultMovedPath = Join-Path `
        ([string]$deleteFaultFixture.AnchorPath) `
        $deleteFaultMovedLeaf
    $deleteFaultObserved = $false
    try {
        $deleteFaultFixture.ProbeDeleteFailedEmptyRootPostOpenFaultForGuardrail()
    } catch {
        Assert-Contains $_.Exception.Message "injected post-open handle fault"
        $deleteFaultObserved = $true
    }
    $deleteFaultHandleClosed = [AmeR2cRDisposableRoot]::GuardrailFaultHandleIsClosed
    $deleteFaultRootHeld = [bool]$deleteFaultFixture.IsRootHandleHeld
    [IO.Directory]::Move($deleteFaultOriginalPath, $deleteFaultMovedPath)
    $deleteFaultFixture.DeleteRelocatedExpectedRootForGuardrail($deleteFaultMovedLeaf)
    $deleteFaultDeleted = -not [IO.Directory]::Exists($deleteFaultMovedPath)
    if (-not $deleteFaultObserved -or
        -not $deleteFaultHandleClosed -or
        $deleteFaultRootHeld -or
        -not $deleteFaultDeleted) {
        throw "R2c-R DeleteFailedEmptyRoot did not release its injected post-open handle"
    }
} finally {
    if ($null -ne $deleteFaultFixture) {
        $deleteFaultFixture.ReleaseDeleteFailedGuardrailFaultHandleForCleanup()
        if ($null -ne $deleteFaultOriginalPath -and
            $null -ne $deleteFaultMovedPath -and
            [IO.Directory]::Exists($deleteFaultOriginalPath) -and
            -not [IO.Directory]::Exists($deleteFaultMovedPath)) {
            [IO.Directory]::Move($deleteFaultOriginalPath, $deleteFaultMovedPath)
        }
        if ($null -ne $deleteFaultMovedPath -and
            [IO.Directory]::Exists($deleteFaultMovedPath)) {
            $deleteFaultFixture.DeleteRelocatedExpectedRootForGuardrail($deleteFaultMovedLeaf)
        }
        $deleteFaultFixture.Dispose()
    }
}

[AmeR2cRDisposableRoot]::AssertOwnedLeafName(
    "Cedarflake-Ame-R2c-R-00000000000000000000000000000000-00000000000000000000000000000000"
)
$maliciousLeaves = @(
    "",
    ".",
    "..",
    "Cedarflake-Ame-R2c-R-name:stream",
    "Cedarflake-Ame-R2c-R-name`0tail",
    "Cedarflake-Ame-R2c-R-name`ttail",
    "Cedarflake-Ame-R2c-R-name/tail",
    "Cedarflake-Ame-R2c-R-name\tail",
    "Cedarflake-Ame-R2c-R-name.",
    "Cedarflake-Ame-R2c-R-name ",
    "Cedarflake-Ame-R2c-R-name",
    "Cedarflake-Ame-R2c-R-00000000000000000000000000000000",
    "Cedarflake-Ame-R2c-R-00000000000000000000000000000000-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "Cedarflake-Ame-R2c-R-相册"
)
foreach ($leaf in $maliciousLeaves) {
    try {
        [AmeR2cRDisposableRoot]::AssertOwnedLeafName($leaf)
        throw "R2c-R unexpectedly accepted a malicious NT leaf"
    } catch {
        if ($_.Exception.Message.IndexOf("relative directory name", [StringComparison]::Ordinal) -lt 0 -and
            $_.Exception.Message.IndexOf("directory component", [StringComparison]::Ordinal) -lt 0) {
            throw
        }
    }
}
$validContext = @{
    IsWindowsPlatform = $true
    OperatingSystemArchitecture = [Runtime.InteropServices.Architecture]::X64
    ProcessArchitecture = [Runtime.InteropServices.Architecture]::X64
    BuildNumber = 22621
    ApiBuildNumber = 22621
    InstallationType = "Client"
    ProductType = "WinNT"
    ProductSku = [uint32]48
    IsAdministrator = $false
}
$currentVersion = Get-AmeR2cRWindowsVersionEvidence
$isCurrentAdministrator = Test-AmeR2cRCurrentProcessIsAdministrator
$isCurrentExecutionContextSupported = $true
try {
    Assert-AmeR2cRExecutionContext `
        -IsWindowsPlatform ([bool]$currentVersion.IsWindows) `
        -OperatingSystemArchitecture (
            [Runtime.InteropServices.RuntimeInformation]::OSArchitecture
        ) `
        -ProcessArchitecture (
            [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
        ) `
        -BuildNumber ([int]$currentVersion.BuildNumber) `
        -ApiBuildNumber ([int]$currentVersion.ApiBuildNumber) `
        -InstallationType ([string]$currentVersion.InstallationType) `
        -ProductType ([string]$currentVersion.ProductType) `
        -ProductSku ([uint32]$currentVersion.ProductSku) `
        -IsAdministrator $isCurrentAdministrator
} catch {
    $isCurrentExecutionContextSupported = $false
}
$executionContextProbes = @(
    [pscustomobject]@{ Label = "non-windows"; Change = @{ IsWindowsPlatform = $false }; Expected = "requires Windows" },
    [pscustomobject]@{ Label = "non-x64-windows"; Change = @{ OperatingSystemArchitecture = [Runtime.InteropServices.Architecture]::Arm64 }; Expected = "requires an x64 process" },
    [pscustomobject]@{ Label = "non-x64-process"; Change = @{ ProcessArchitecture = [Runtime.InteropServices.Architecture]::X86 }; Expected = "requires an x64 process" },
    [pscustomobject]@{ Label = "windows-10"; Change = @{ BuildNumber = 19045; ApiBuildNumber = 19045 }; Expected = "requires Windows 11" },
    [pscustomobject]@{ Label = "server-installation"; Change = @{ InstallationType = "Server"; ProductType = "ServerNT" }; Expected = "client workstation SKU" },
    [pscustomobject]@{ Label = "server-product-type"; Change = @{ ProductType = "ServerNT" }; Expected = "client workstation SKU" },
    [pscustomobject]@{ Label = "unresolved-sku"; Change = @{ ProductSku = [uint32]0 }; Expected = "resolved Windows client SKU" },
    [pscustomobject]@{ Label = "build-mismatch"; Change = @{ ApiBuildNumber = 22631 }; Expected = "build evidence disagrees" },
    [pscustomobject]@{ Label = "administrator"; Change = @{ IsAdministrator = $true }; Expected = "ordinary non-administrator user" }
)
foreach ($probe in $executionContextProbes) {
    $arguments = @{}
    foreach ($key in $validContext.Keys) { $arguments[$key] = $validContext[$key] }
    foreach ($key in $probe.Change.Keys) { $arguments[$key] = $probe.Change[$key] }
    try {
        Assert-AmeR2cRExecutionContext @arguments
        throw "R2c-R unexpectedly accepted the $($probe.Label) context"
    } catch { Assert-Contains $_.Exception.Message $probe.Expected }
}
Assert-AmeR2cRExecutionContext @validContext

Assert-AmeR2cRCaseMatrix -RepositoryRoot $repositoryRoot
$cases = @(Get-AmeR2cRCaseDefinitions)
if ($cases.Count -ne 19 -or @($cases | Where-Object { $_.Ignored }).Count -ne 4 -or
    @($cases | Where-Object { -not $_.Ignored }).Count -ne 15) {
    throw "R2c-R guardrail did not retain the exact 19-case, 15-normal, four-ignored matrix"
}
foreach ($case in $cases) {
    $arguments = @(Get-AmeR2cRCaseArguments -Case $case)
    if (@($arguments | Where-Object { $_ -ceq "--exact" }).Count -ne 1) {
        throw "R2c-R case '$($case.Label)' lost its exact selector"
    }
}

try {
    Assert-AmeR2cRCaseResult -Label "zero-tests" -Lines @(
        "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 872 filtered out;"
    ) -ExitCode 0
    throw "R2c-R unexpectedly accepted a zero-test result"
} catch { Assert-Contains $_.Exception.Message "did not execute exactly one passing" }
Assert-AmeR2cRCaseResult -Label "one-test" -Lines @(
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 872 filtered out;"
) -ExitCode 0

$anchorAttackFixture = $null
$anchorAttackFixturePath = $null
$anchorSentinelFixture = $null
$anchorSentinelFixturePath = $null
$anchorSentinelStream = $null
$anchorSentinelChildLeaf = $null
$anchorSentinelChildPath = $null
$anchorSentinelChildIdentity = $null
$junctionResources = @()
try {
    $anchorAttackFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $anchorAttackFixturePath = [string]$anchorAttackFixture.Path
    $anchorSentinelFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $anchorSentinelFixturePath = [string]$anchorSentinelFixture.Path
    $anchorAttackFixture.ValidateHeldIdentity()
    $anchorSentinelFixture.ValidateHeldIdentity()
    if (-not [bool]$anchorAttackFixture.IsRootHandleHeld) {
        throw "R2c-R disposable root did not retain its replacement-blocking handle"
    }
    if ([string]$anchorAttackFixture.RootName -notmatch
        '^Cedarflake-Ame-R2c-R-[0-9a-f]{32}-[0-9a-f]{32}$') {
        throw "R2c-R disposable root did not use a high-entropy direct-child name"
    }

    $anchorSentinelChildLeaf = "anchor-$([Guid]::NewGuid().ToString('N'))"
    $anchorSentinelChildIdentity = $anchorSentinelFixture.CreateGuardrailChildDirectory(
        $anchorSentinelChildLeaf
    )
    $anchorSentinelChildPath = Join-Path `
        ([string]$anchorSentinelFixture.Path) `
        $anchorSentinelChildLeaf
    $anchorSentinelStream = [IO.FileStream]::new(
        (Join-Path (
            $anchorSentinelChildPath
        ) "sentinel.bin"),
        [IO.FileMode]::CreateNew,
        [IO.FileAccess]::ReadWrite,
        ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
        4096,
        [IO.FileOptions]::DeleteOnClose
    )
    $anchorSentinelStream.Write([byte[]](9, 10, 11, 12), 0, 4)
    $anchorSentinelStream.Flush($true)
    $sentinelBefore = @(Get-TreeShape -Path ([string]$anchorSentinelFixture.Path))

    foreach ($label in @("terminal", "owned-terminal")) {
        $leaf = "$label-$([Guid]::NewGuid().ToString('N'))"
        $candidate = Join-Path ([string]$anchorAttackFixture.Path) $leaf
        New-Item -ItemType Junction -Path $candidate -Target $anchorSentinelChildPath | Out-Null
        $identity = $anchorAttackFixture.CaptureGuardrailChildDirectoryIdentity($leaf, $true)
        $junctionResources += [pscustomobject]@{ Leaf = $leaf; Identity = $identity }
        try {
            [AmeR2cRNative]::AssertDirectoryAnchor($candidate)
            throw "R2c-R unexpectedly trusted the internal junction candidate"
        } catch {
            Assert-Contains $_.Exception.Message "reparse point"
        }
        $failedChild = Join-Path $candidate "Cedarflake-Ame-R2c-R-failed-create"
        if ([IO.Directory]::Exists($failedChild) -or [IO.File]::Exists($failedChild)) {
            throw "R2c-R junction rejection left a failed child residue"
        }
        if (((Get-TreeShape -Path ([string]$anchorSentinelFixture.Path)) -join "`n") -cne
            ($sentinelBefore -join "`n")) {
            throw "R2c-R junction rejection wrote through to the internal sentinel"
        }
    }

    $ancestorLeaf = "ancestor-$([Guid]::NewGuid().ToString('N'))"
    $ancestorPath = Join-Path ([string]$anchorAttackFixture.Path) $ancestorLeaf
    New-Item -ItemType Junction `
        -Path $ancestorPath `
        -Target ([string]$anchorSentinelFixture.Path) | Out-Null
    $ancestorIdentity = $anchorAttackFixture.CaptureGuardrailChildDirectoryIdentity(
        $ancestorLeaf,
        $true
    )
    $junctionResources += [pscustomobject]@{
        Leaf = $ancestorLeaf
        Identity = $ancestorIdentity
    }
    $knownFolderAttackPath = Join-Path $ancestorPath $anchorSentinelChildLeaf
    try {
        [AmeR2cRDisposableRoot]::CreateForKnownFolderGuardrail(
            $knownFolderAttackPath,
            [Guid]::NewGuid().ToString("N")
        ) | Out-Null
        throw "R2c-R unexpectedly accepted a KnownFolder intermediate junction"
    } catch {
        Assert-Contains $_.Exception.Message "reparse point"
    }
    if (((Get-TreeShape -Path ([string]$anchorSentinelFixture.Path)) -join "`n") -cne
        ($sentinelBefore -join "`n")) {
        throw "R2c-R KnownFolder ancestor rejection wrote before physical binding"
    }
} finally {
    if ($null -ne $anchorAttackFixture) {
        foreach ($resource in @($junctionResources)) {
            $anchorAttackFixture.DeleteGuardrailChildDirectoryForCleanup(
                [string]$resource.Leaf,
                [string]$resource.Identity,
                $true
            )
        }
        Remove-AmeR2cRDisposableRoot -Fixture $anchorAttackFixture
    }
    if ($null -ne $anchorSentinelStream) { $anchorSentinelStream.Dispose() }
    if ($null -ne $anchorSentinelFixture) {
        if (-not [string]::IsNullOrEmpty($anchorSentinelChildIdentity)) {
            $anchorSentinelFixture.DeleteGuardrailChildDirectoryForCleanup(
                $anchorSentinelChildLeaf,
                $anchorSentinelChildIdentity,
                $false
            )
        }
        Remove-AmeR2cRDisposableRoot -Fixture $anchorSentinelFixture
    }
}
if ($null -ne $anchorAttackFixturePath -and [IO.Directory]::Exists($anchorAttackFixturePath)) {
    throw "R2c-R anchor-attack guardrail left a disposable-root residue"
}
if ($null -ne $anchorSentinelFixturePath -and [IO.Directory]::Exists($anchorSentinelFixturePath)) {
    throw "R2c-R anchor sentinel guardrail left a disposable-root residue"
}

foreach ($replacementKind in @("ordinary", "junction")) {
    $raceFixture = $null
    $raceTargetFixture = $null
    $raceTargetFixturePath = $null
    $raceTargetStream = $null
    $raceOriginal = $null
    $raceMoved = $null
    $raceMovedLeaf = $null
    $raceReplacementMoved = $null
    $raceReplacementMovedLeaf = $null
    $raceState = $null
    try {
        if ($replacementKind -ceq "junction") {
            $raceTargetFixture = New-AmeR2cRGuardrailDisposableRoot `
                -AnchorPath $toolPath `
                -Nonce ([Guid]::NewGuid().ToString("N"))
            $raceTargetFixturePath = [string]$raceTargetFixture.Path
            $raceTargetStream = [IO.FileStream]::new(
                (Join-Path $raceTargetFixturePath "sentinel.bin"),
                [IO.FileMode]::CreateNew,
                [IO.FileAccess]::ReadWrite,
                ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
                4096,
                [IO.FileOptions]::DeleteOnClose
            )
            $raceTargetStream.Write([byte[]](13, 14, 15, 16), 0, 4)
            $raceTargetStream.Flush($true)
            $raceSentinelBefore = @(Get-TreeShape -Path $raceTargetFixturePath)
        }
        $raceFixture = New-AmeR2cRGuardrailDisposableRoot `
            -AnchorPath $toolPath `
            -Nonce ([Guid]::NewGuid().ToString("N"))
        $raceOriginal = [string]$raceFixture.Path
        $raceIdentity = [string]$raceFixture.RootIdentityToken
        $raceMovedLeaf = "$($raceFixture.RootName)-moved-$([Guid]::NewGuid().ToString('N'))"
        $raceMoved = Join-Path ([string]$raceFixture.AnchorPath) $raceMovedLeaf
        $raceReplacementMovedLeaf = "$($raceFixture.RootName)-moved-$([Guid]::NewGuid().ToString('N'))"
        $raceReplacementMoved = Join-Path `
            ([string]$raceFixture.AnchorPath) `
            $raceReplacementMovedLeaf
        $raceState = [pscustomobject]@{
            MovedCreated = $false
            ReplacementCreated = $false
            ReplacementIdentity = $null
            ReplacementMoved = $false
            UnknownCreated = $false
            UnknownIdentity = $null
        }
        Remove-AmeR2cRDisposableRoot `
            -Fixture $raceFixture `
            -RetainFixtureOnFailureForGuardrail `
            -ReleaseRootBlockerOnlyForGuardrail
        [IO.Directory]::Move($raceOriginal, $raceMoved)
        $raceState.MovedCreated = $true
        if ($replacementKind -ceq "junction") {
            New-Item `
                -ItemType Junction `
                -Path $raceOriginal `
                -Target $raceTargetFixturePath | Out-Null
        } else {
            [IO.Directory]::CreateDirectory($raceOriginal) | Out-Null
        }
        $raceState.ReplacementIdentity = $raceFixture.CaptureGuardrailReplacementIdentity(
            $raceFixture.RootName,
            ($replacementKind -ceq "junction")
        )
        $raceState.ReplacementCreated = $true
        $cleanupRaceMessage = $null
        try {
            Remove-AmeR2cRDisposableRoot `
                -Fixture $raceFixture `
                -RetainFixtureOnFailureForGuardrail `
                -DeleteReleasedRootForGuardrail
            throw "R2c-R cleanup unexpectedly deleted a $replacementKind replacement"
        } catch {
            $cleanupRaceMessage = $_.Exception.Message
            Assert-Contains $_.Exception.Message "no path fallback was attempted"
        }
        if ([bool]$raceFixture.IsRootHandleHeld) {
            throw "R2c-R cleanup race did not enter the post-blocker-release boundary"
        }
        if ($replacementKind -ceq "junction") {
            Assert-Contains $cleanupRaceMessage "reparse point"
        } else {
            Assert-Contains $cleanupRaceMessage "identity changed"
        }
        Assert-Contains $cleanupRaceMessage "owned_leaf=$($raceFixture.RootName)"
        Assert-Contains $cleanupRaceMessage "expected_identity=$raceIdentity"
        if (-not [IO.Directory]::Exists($raceOriginal) -or
            -not [IO.Directory]::Exists($raceMoved)) {
            throw "R2c-R cleanup changed a $replacementKind replacement or moved original"
        }
        if ($replacementKind -ceq "junction" -and
            ((Get-TreeShape -Path $raceTargetFixturePath) -join "`n") -cne
                ($raceSentinelBefore -join "`n")) {
            throw "R2c-R cleanup traversed a junction replacement sentinel"
        }

        [IO.Directory]::Move($raceOriginal, $raceReplacementMoved)
        $raceState.ReplacementMoved = $true
        $raceState.ReplacementCreated = $false
        if ($replacementKind -ceq "junction") {
            New-Item `
                -ItemType Junction `
                -Path $raceOriginal `
                -Target $raceTargetFixturePath | Out-Null
        } else {
            [IO.Directory]::CreateDirectory($raceOriginal) | Out-Null
        }
        $raceState.UnknownIdentity = $raceFixture.CaptureGuardrailReplacementIdentity(
            $raceFixture.RootName,
            ($replacementKind -ceq "junction")
        )
        $raceState.UnknownCreated = $true
        try {
            $raceFixture.DeleteGuardrailReplacementForCleanup(
                $raceFixture.RootName,
                [string]$raceState.ReplacementIdentity,
                ($replacementKind -ceq "junction")
            )
            throw "R2c-R teardown unexpectedly deleted a second $replacementKind swap"
        } catch {
            Assert-Contains $_.Exception.Message "identity changed before teardown"
        }
        if (-not [IO.Directory]::Exists($raceOriginal)) {
            throw "R2c-R teardown deleted an unknown second $replacementKind replacement"
        }
        if ($replacementKind -ceq "junction" -and
            ((Get-TreeShape -Path $raceTargetFixturePath) -join "`n") -cne
                ($raceSentinelBefore -join "`n")) {
            throw "R2c-R teardown traversed the second junction swap sentinel"
        }
        $raceFixture.DeleteGuardrailReplacementForCleanup(
            $raceFixture.RootName,
            [string]$raceState.UnknownIdentity,
            ($replacementKind -ceq "junction")
        )
        $raceState.UnknownCreated = $false
        $raceFixture.DeleteGuardrailReplacementForCleanup(
            $raceReplacementMovedLeaf,
            [string]$raceState.ReplacementIdentity,
            ($replacementKind -ceq "junction")
        )
        $raceState.ReplacementMoved = $false
        $raceFixture.DeleteRelocatedExpectedRootForGuardrail($raceMovedLeaf)
        $raceState.MovedCreated = $false
    } finally {
        if ($null -ne $raceFixture) {
            if ($null -ne $raceState -and [bool]$raceState.UnknownCreated) {
                $raceFixture.DeleteGuardrailReplacementForCleanup(
                    $raceFixture.RootName,
                    [string]$raceState.UnknownIdentity,
                    ($replacementKind -ceq "junction")
                )
            }
            if ($null -ne $raceState -and [bool]$raceState.ReplacementCreated) {
                $raceFixture.DeleteGuardrailReplacementForCleanup(
                    $raceFixture.RootName,
                    [string]$raceState.ReplacementIdentity,
                    ($replacementKind -ceq "junction")
                )
            }
            if ($null -ne $raceState -and [bool]$raceState.ReplacementMoved) {
                $raceFixture.DeleteGuardrailReplacementForCleanup(
                    $raceReplacementMovedLeaf,
                    [string]$raceState.ReplacementIdentity,
                    ($replacementKind -ceq "junction")
                )
            }
            if ($null -ne $raceState -and [bool]$raceState.MovedCreated) {
                $raceFixture.DeleteRelocatedExpectedRootForGuardrail($raceMovedLeaf)
            }
            $raceFixture.Dispose()
        }
        if ($null -ne $raceTargetStream) { $raceTargetStream.Dispose() }
        if ($null -ne $raceTargetFixture) {
            Remove-AmeR2cRDisposableRoot -Fixture $raceTargetFixture
        }
    }
}

$timeoutFixture = $null
$sideProcess = $null
$timeoutFixturePath = $null
$timeoutOutputPath = $null
try {
    $timeoutFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $timeoutFixturePath = [string]$timeoutFixture.Path
    $hostExecutable = (Get-Process -Id $PID).Path
    $sideResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments @(
            "-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"
        ) `
        -WorkingDirectory $repositoryRoot `
        -Detached
    $sideProcess = $sideResult.Process
    $escapedTimeoutCommon = $commonPath.Replace("'", "''")
    $escapedTimeoutRepository = $repositoryRoot.Replace("'", "''")
    $blockingCommand = @"
`$ErrorActionPreference = "Stop"
`$timeoutClock = [Diagnostics.Stopwatch]::StartNew()
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=before-common elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
. '$escapedTimeoutCommon'
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=after-common elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=before-native elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
Initialize-AmeR2cRNativeTypes -RepositoryRoot '$escapedTimeoutRepository'
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=after-native elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
`$hostExecutable = (Get-Process -Id `$PID).Path
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=before-child-start elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
`$detached = Invoke-AmeR2cROwnedProcess ``
    -ExecutableKind "PowerShell" ``
    -FileName `$hostExecutable ``
    -Arguments @("-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30") ``
    -WorkingDirectory '$escapedTimeoutRepository' ``
    -Detached
`$child = `$detached.Process
[Console]::Out.WriteLine("R2C_R_TIMEOUT_PHASE phase=after-child-start elapsed_ms=`$(`$timeoutClock.ElapsedMilliseconds)")
[Console]::Out.Flush()
Write-Output "OWNED_CHILD_PID=`$(`$child.Id)"
[Console]::Out.Flush()
try { `$child.WaitForExit() } finally { `$child.Dispose() }
"@
    $timeoutOutputPath = Join-Path ([string]$timeoutFixture.Path) "blocked-process.log"
    $timeoutResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "PowerShell" `
        -FileName $hostExecutable `
        -Arguments @("-NoProfile", "-NonInteractive", "-Command", $blockingCommand) `
        -WorkingDirectory $repositoryRoot `
        -OutputPath $timeoutOutputPath `
        -TimeoutMilliseconds 15000
    if (-not $timeoutResult.TimedOut) {
        throw (
            "R2c-R blocking fixture unexpectedly completed before the parent timeout; " +
            "exit=$($timeoutResult.ExitCode); lines=$($timeoutResult.Lines -join ' | ')"
        )
    }
    $childMarker = @($timeoutResult.Lines | Where-Object { $_ -match '^OWNED_CHILD_PID=[0-9]+$' })
    if ($childMarker.Count -ne 1) {
        throw (
            "R2c-R blocking fixture did not report exactly one owned child; " +
            "TimedOut=$($timeoutResult.TimedOut); ExitCode=$($timeoutResult.ExitCode); " +
            "ProcessId=$($timeoutResult.ProcessId); markerCount=$($childMarker.Count); " +
            "Lines=$($timeoutResult.Lines -join ' | ')"
        )
    }
    $ownedChildId = [int]($childMarker[0] -replace '^OWNED_CHILD_PID=', '')
    $ownedChild = Get-Process -Id $ownedChildId -ErrorAction SilentlyContinue
    if ($null -ne $ownedChild) {
        $ownedChild.Dispose()
        throw "R2c-R timeout cleanup left an owned descendant running"
    }
    $sideProcess.Refresh()
    if ($sideProcess.HasExited) {
        throw "R2c-R timeout cleanup terminated a process outside its owned job tree"
    }
} finally {
    if ($null -ne $sideProcess -and -not $sideProcess.HasExited) {
        Stop-Process -Id $sideProcess.Id -Force
        $sideProcess.WaitForExit()
    }
    if ($null -ne $sideProcess) { $sideProcess.Dispose() }
    if ($null -ne $timeoutFixture) { Remove-AmeR2cRDisposableRoot -Fixture $timeoutFixture }
}
if ($null -ne $timeoutFixturePath -and [IO.Directory]::Exists($timeoutFixturePath)) {
    throw "R2c-R timeout guardrail did not clean its disposable fixture"
}

$tamperCase = @(
    Get-AmeR2cRCaseDefinitions |
        Where-Object { $_.Label -ceq "worker-report-binding-tamper" }
)
if ($tamperCase.Count -ne 1 -or [bool]$tamperCase[0].Ignored) {
    throw "R2c-R guardrail lost the executable worker-report tamper case"
}
$tamperFixture = $null
$tamperOutputPath = $null
$tamperToolLock = $null
$previousCargoJobs = [Environment]::GetEnvironmentVariable("CARGO_BUILD_JOBS", "Process")
$previousRustThreads = [Environment]::GetEnvironmentVariable("RUST_TEST_THREADS", "Process")
try {
    $tamperFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $tamperOutputPath = Join-Path ([string]$tamperFixture.Path) "tamper-behavior.log"
    [Environment]::SetEnvironmentVariable("CARGO_BUILD_JOBS", "1", "Process")
    [Environment]::SetEnvironmentVariable("RUST_TEST_THREADS", "1", "Process")
    $tamperToolLock = Enter-AmeRepositoryToolLock
    $tamperResult = Invoke-AmeR2cROwnedProcess `
        -ExecutableKind "CargoExactTest" `
        -FileName ((Get-AmeToolchain).Cargo) `
        -Arguments @(Get-AmeR2cRCaseArguments -Case $tamperCase[0]) `
        -WorkingDirectory $repositoryRoot `
        -OutputPath $tamperOutputPath `
        -TimeoutMilliseconds 300000
    if ($tamperResult.TimedOut) {
        throw "R2c-R worker-report tamper behavior test exceeded its five-minute deadline"
    }
    Assert-AmeR2cRCaseResult `
        -Label "worker-report-binding-tamper-lint" `
        -Lines $tamperResult.Lines `
        -ExitCode $tamperResult.ExitCode
} finally {
    if ($null -ne $tamperToolLock) { Exit-AmeRepositoryToolLock $tamperToolLock }
    if ($null -eq $previousCargoJobs) {
        [Environment]::SetEnvironmentVariable(
            "CARGO_BUILD_JOBS",
            $nullEnvironmentValue,
            [EnvironmentVariableTarget]::Process
        )
    } else {
        [Environment]::SetEnvironmentVariable(
            "CARGO_BUILD_JOBS",
            $previousCargoJobs,
            [EnvironmentVariableTarget]::Process
        )
    }
    if ($null -eq $previousRustThreads) {
        [Environment]::SetEnvironmentVariable(
            "RUST_TEST_THREADS",
            $nullEnvironmentValue,
            [EnvironmentVariableTarget]::Process
        )
    } else {
        [Environment]::SetEnvironmentVariable(
            "RUST_TEST_THREADS",
            $previousRustThreads,
            [EnvironmentVariableTarget]::Process
        )
    }
    if ($null -ne $tamperFixture) { Remove-AmeR2cRDisposableRoot -Fixture $tamperFixture }
}
$restoredCargoJobs = [Environment]::GetEnvironmentVariable("CARGO_BUILD_JOBS", "Process")
$restoredRustThreads = [Environment]::GetEnvironmentVariable("RUST_TEST_THREADS", "Process")
if (($null -eq $previousCargoJobs) -ne ($null -eq $restoredCargoJobs) -or
    ($null -ne $previousCargoJobs -and $previousCargoJobs -cne $restoredCargoJobs) -or
    ($null -eq $previousRustThreads) -ne ($null -eq $restoredRustThreads) -or
    ($null -ne $previousRustThreads -and $previousRustThreads -cne $restoredRustThreads)) {
    throw "R2c-R tamper fixture did not restore the Cargo and Rust process environment"
}

$runner = Join-Path $PSScriptRoot "acceptance_run_r2c_change_driven_reliability.ps1"
$environmentNames = @(Get-AmeR2cRForbiddenEnvironmentNames)
$preservedNames = @($environmentNames + @("PUBLIC", "TEMP", "TMP") | Sort-Object -Unique)
$previousEnvironment = @{}
$runnerFixture = $null
foreach ($name in $preservedNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
try {
    foreach ($name in $preservedNames) {
        [Environment]::SetEnvironmentVariable(
            $name,
            $nullEnvironmentValue,
            [EnvironmentVariableTarget]::Process
        )
    }
    $unclearedEnvironment = @(
        $preservedNames |
            Where-Object {
                $null -ne [Environment]::GetEnvironmentVariable($_, "Process")
            }
    )
    if ($unclearedEnvironment.Count -ne 0) {
        throw (
            "R2c-R guardrail could not delete inherited runner environment values: " +
            ($unclearedEnvironment -join ", ")
        )
    }
    $runnerFixture = New-AmeR2cRGuardrailDisposableRoot `
        -AnchorPath $toolPath `
        -Nonce ([Guid]::NewGuid().ToString("N"))
    $invalidStorage = Invoke-R2cRGuardrailRunnerProcess `
        -RunnerPath $runner `
        -RunnerArguments @("-GuardrailWorkspaceAnchor") `
        -RepositoryRoot $repositoryRoot `
        -OutputRoot ([string]$runnerFixture.Path)
    if ($invalidStorage.TimedOut -or $invalidStorage.ExitCode -eq 0) {
        throw "R2c-R unexpectedly admitted workspace guardrail storage to a full run"
    }
    $invalidStorageTokens = @(
        $invalidStorage.Lines |
            Where-Object {
                $_ -ceq (
                    "AME_R2C_R_REJECTION " +
                    "reason=workspace-requires-validation-only"
                )
            }
    )
    if ($invalidStorageTokens.Count -ne 1) {
        throw (
            "R2c-R workspace rejection did not report one stable reason token; " +
            "exit=$($invalidStorage.ExitCode) timeout=$($invalidStorage.TimedOut) " +
            "lines=$($invalidStorage.Lines -join ' | ')"
        )
    }
    foreach ($parameter in @("SourceRoot", "LocalRoot", "CloudRoot", "SourceCatalogPath")) {
        $invalidAlias = Invoke-R2cRGuardrailRunnerProcess `
            -RunnerPath $runner `
            -RunnerArguments @(
                "-ValidationOnly", "-$parameter", "C:\external-library-alias"
            ) `
            -RepositoryRoot $repositoryRoot `
            -OutputRoot ([string]$runnerFixture.Path)
        if ($invalidAlias.TimedOut -or $invalidAlias.ExitCode -eq 0) {
            throw "R2c-R unexpectedly accepted the caller-supplied $parameter"
        }
        $invalidAliasTokens = @(
            $invalidAlias.Lines |
                Where-Object {
                    $_ -ceq "AME_R2C_R_REJECTION reason=caller-supplied-source"
                }
        )
        if ($invalidAliasTokens.Count -ne 1) {
            throw (
                "R2c-R $parameter rejection did not report one stable reason token; " +
                "exit=$($invalidAlias.ExitCode) timeout=$($invalidAlias.TimedOut) " +
                "lines=$($invalidAlias.Lines -join ' | ')"
            )
        }
    }
    foreach ($name in $environmentNames) {
        [Environment]::SetEnvironmentVariable($name, "C:\external-environment-alias", "Process")
        try {
            $invalidEnvironment = Invoke-R2cRGuardrailRunnerProcess `
                -RunnerPath $runner `
                -RunnerArguments @("-ValidationOnly") `
                -RepositoryRoot $repositoryRoot `
                -OutputRoot ([string]$runnerFixture.Path)
            if ($invalidEnvironment.TimedOut -or $invalidEnvironment.ExitCode -eq 0) {
                throw "R2c-R unexpectedly accepted environment alias $name"
            }
            $invalidEnvironmentTokens = @(
                $invalidEnvironment.Lines |
                    Where-Object {
                        $_ -ceq (
                            "AME_R2C_R_REJECTION " +
                            "reason=caller-supplied-environment"
                        )
                    }
            )
            if ($invalidEnvironmentTokens.Count -ne 1) {
                throw (
                    "R2c-R $name rejection did not report one stable reason token; " +
                    "exit=$($invalidEnvironment.ExitCode) " +
                    "timeout=$($invalidEnvironment.TimedOut) " +
                    "lines=$($invalidEnvironment.Lines -join ' | ')"
                )
            }
        }
        finally {
            [Environment]::SetEnvironmentVariable(
                $name,
                $nullEnvironmentValue,
                [EnvironmentVariableTarget]::Process
            )
        }
    }

    [Environment]::SetEnvironmentVariable("PUBLIC", "C:\hostile-public-alias", "Process")
    [Environment]::SetEnvironmentVariable("TEMP", "C:\hostile-temp-alias", "Process")
    [Environment]::SetEnvironmentVariable("TMP", "C:\hostile-tmp-alias", "Process")
    $valid = Invoke-R2cRGuardrailRunnerProcess `
        -RunnerPath $runner `
        -RunnerArguments @("-ValidationOnly", "-GuardrailWorkspaceAnchor") `
        -RepositoryRoot $repositoryRoot `
        -OutputRoot ([string]$runnerFixture.Path)
    $validText = $valid.Lines | Out-String
    if ($isCurrentExecutionContextSupported) {
        if ($valid.TimedOut -or $valid.ExitCode -ne 0) {
            throw (
                "R2c-R valid runner process failed; exit=$($valid.ExitCode) " +
                "timeout=$($valid.TimedOut) lines=$($valid.Lines -join ' | ')"
            )
        }
        Assert-Contains $validText "AME_R2C_R_VALIDATION status=passed"
        Assert-Contains $validText "cases=19 normal=15 ignored=4"
        Assert-Contains $validText "selectors=exact"
        Assert-Contains $validText "root_physical=held-handle"
        Assert-Contains $validText "platform=windows11-x64"
        Assert-Contains $validText "phase=validation"
    } else {
        if ($valid.TimedOut -or $valid.ExitCode -eq 0) {
            throw (
                "R2c-R unsupported host did not reject the validation runner; " +
                "exit=$($valid.ExitCode) timeout=$($valid.TimedOut) " +
                "lines=$($valid.Lines -join ' | ')"
            )
        }
        $executionContextTokens = @(
            $valid.Lines |
                Where-Object {
                    $_ -ceq "AME_R2C_R_REJECTION reason=execution-context"
                }
        )
        if ($executionContextTokens.Count -ne 1 -or
            $validText.IndexOf(
                "AME_R2C_R_VALIDATION status=passed",
                [StringComparison]::Ordinal
            ) -ge 0) {
            throw (
                "R2c-R unsupported host rejection was not fail-closed; " +
                "exit=$($valid.ExitCode) timeout=$($valid.TimedOut) " +
                "lines=$($valid.Lines -join ' | ')"
            )
        }
    }
    if ([Environment]::GetEnvironmentVariable("TEMP", "Process") -cne "C:\hostile-temp-alias") {
        throw "R2c-R ValidationOnly did not restore the caller TEMP value"
    }
    if ([Environment]::GetEnvironmentVariable("TMP", "Process") -cne "C:\hostile-tmp-alias") {
        throw "R2c-R ValidationOnly did not restore the caller TMP value"
    }
} finally {
    foreach ($name in $preservedNames) {
        if ($null -eq $previousEnvironment[$name]) {
            [Environment]::SetEnvironmentVariable(
                $name,
                $nullEnvironmentValue,
                [EnvironmentVariableTarget]::Process
            )
        } else {
            [Environment]::SetEnvironmentVariable(
                $name,
                $previousEnvironment[$name],
                [EnvironmentVariableTarget]::Process
            )
        }
    }
    if ($null -ne $runnerFixture) {
        Remove-AmeR2cRDisposableRoot -Fixture $runnerFixture
    }
}

$runnerText = [IO.File]::ReadAllText($runner)
$commonText = [IO.File]::ReadAllText($commonPath)
$guardrailText = [IO.File]::ReadAllText($PSCommandPath)
Assert-AmeR2cRDeletionSafetySource `
    -SourceText $commonText `
    -SourcePath $commonPath `
    -Label "common module"
$runnerClosureState = $null
try {
    $runnerClosureState = Assert-AmeR2cRDeletionSafetySource `
        -SourceText $runnerText `
        -SourcePath $runner `
        -Label "acceptance runner" `
        -RetainSnapshots
    $runnerDepth = @(
        $runnerClosureState.Sources |
            Select-Object -ExpandProperty Depth |
            Sort-Object -Descending |
            Select-Object -First 1
    )
    if ($runnerClosureState.Sources.Count -ne 3 -or
        $runnerDepth.Count -ne 1 -or
        [uint64]$runnerDepth[0] -ne 1) {
        throw "R2c-R real closure lost its three-source, depth-one budget evidence"
    }
} finally {
    if ($null -ne $runnerClosureState) {
        Close-AmeR2cRDeletionAuditState -State $runnerClosureState
    }
}
Assert-AmeR2cRDeletionSafetySource `
    -SourceText $guardrailText `
    -SourcePath $PSCommandPath `
    -Label "guardrail"

$sixthReviewDeletionAuditBadFixtures = @(
    [pscustomobject]@{
        Label = "top-level call-operator variable"
        Source = '$command = "Write-Output"; & $command "owned"'
        Expected = "unresolved command invocation"
    },
    [pscustomobject]@{
        Label = "helper call-operator variable"
        Source = 'function Invoke-Helper { $command = "Write-Output"; & $command "owned" }'
        Expected = "unresolved command invocation"
    },
    [pscustomobject]@{
        Label = "cleanup owner call-operator variable"
        Source = 'function Remove-AmeR2cRDisposableRoot { $command = "Write-Output"; & $command "owned" }'
        Expected = "unresolved command invocation"
    },
    [pscustomobject]@{
        Label = "top-level variable dot-source"
        Source = '$loader = ".\unknown.ps1"; . $loader'
        Expected = "unresolved dot-source"
    },
    [pscustomobject]@{
        Label = "helper variable dot-source"
        Source = 'function Import-Helper { $loader = ".\unknown.ps1"; . $loader }'
        Expected = "unresolved dot-source"
    },
    [pscustomobject]@{
        Label = "cmd rmdir"
        Source = 'cmd.exe /d /s /c "rmdir /s /q owned"'
        Expected = "external executable"
    },
    [pscustomobject]@{
        Label = "unregistered EncodedCommand"
        Source = 'powershell.exe -NoProfile -EncodedCommand $encoded'
        Expected = "external executable"
    }
)
foreach ($badFixture in $sixthReviewDeletionAuditBadFixtures) {
    try {
        Assert-AmeR2cRDeletionSafetySource `
            -SourceText ([string]$badFixture.Source) `
            -Label ([string]$badFixture.Label)
        throw "R2c-R deletion safety audit accepted sixth-review fixture '$($badFixture.Label)'"
    } catch {
        Assert-Contains $_.Exception.Message ([string]$badFixture.Expected)
    }
}

$deletionAuditBadFixtures = @(
    [pscustomobject]@{
        Label = "module-qualified Remove-Item"
        Source = @'
function Remove-AmeR2cRDisposableRoot {
    Microsoft.PowerShell.Management\Remove-Item -LiteralPath "owned"
}
'@
        Child = @()
        Expected = "command 'Remove-Item'"
    },
    [pscustomobject]@{
        Label = "dynamic cleanup command"
        Source = @'
function Remove-AmeR2cRDisposableRoot {
    $command = "Write-Output"
    & $command "owned"
}
'@
        Child = @()
        Expected = "unresolved command invocation"
    },
    [pscustomobject]@{
        Label = "alias command"
        Source = @'
function Remove-AmeR2cRDisposableRoot {
    ri -LiteralPath "owned"
}
'@
        Child = @()
        Expected = "command 'ri'"
    },
    [pscustomobject]@{
        Label = "EncodedCommand File.Delete"
        Source = 'powershell.exe -NoProfile -EncodedCommand $encoded'
        Child = @('[IO.File]::Delete("owned")')
        Expected = "external executable"
    },
    [pscustomobject]@{
        Label = "EncodedCommand Directory.Delete"
        Source = 'powershell.exe -NoProfile -EncodedCommand $encoded'
        Child = @('[IO.Directory]::Delete("owned")')
        Expected = "external executable"
    },
    [pscustomobject]@{
        Label = "EncodedCommand Remove-Item"
        Source = 'powershell.exe -NoProfile -EncodedCommand $encoded'
        Child = @('Remove-Item -LiteralPath "owned"')
        Expected = "external executable"
    },
    [pscustomobject]@{
        Label = "ScriptBlock.Create"
        Source = '[ScriptBlock]::Create("Write-Output owned")'
        Child = @()
        Expected = "ScriptBlock.Create"
    },
    [pscustomobject]@{
        Label = "reflection Invoke"
        Source = '$method.Invoke($null, @())'
        Child = @()
        Expected = "reflection invocation"
    },
    [pscustomobject]@{
        Label = "owner to unknown helper"
        Source = @'
function Remove-AmeR2cRDisposableRoot {
    Invoke-UnknownCleanup "owned"
}
'@
        Child = @()
        Expected = "unresolved helper"
    }
)
foreach ($badFixture in $deletionAuditBadFixtures) {
    try {
        Assert-AmeR2cRDeletionSafetySource `
            -SourceText ([string]$badFixture.Source) `
            -Label ([string]$badFixture.Label)
        throw "R2c-R deletion safety audit accepted bad fixture '$($badFixture.Label)'"
    } catch {
        Assert-Contains $_.Exception.Message ([string]$badFixture.Expected)
    }
}
$additionalDeletionAuditBadFixtures = @(
    [pscustomobject]@{ Label = "Remove-Item rm alias"; Source = 'rm -LiteralPath "owned"'; Expected = "command 'rm'" },
    [pscustomobject]@{ Label = "Remove-Item rmdir alias"; Source = 'rmdir "owned"'; Expected = "command 'rmdir'" },
    [pscustomobject]@{ Label = "Set-Alias definition"; Source = 'Set-Alias -Name cleanup -Value Remove-Item'; Expected = "command 'Set-Alias'" },
    [pscustomobject]@{ Label = "New-Alias definition"; Source = 'New-Alias -Name cleanup -Value Remove-Item'; Expected = "command 'New-Alias'" },
    [pscustomobject]@{ Label = "cleanup owner variable dot-source"; Source = 'function Remove-AmeR2cRDisposableRoot { $loader = ".\unknown.ps1"; . $loader }'; Expected = "unresolved dot-source" },
    [pscustomobject]@{ Label = "Invoke-Expression"; Source = 'Invoke-Expression "Write-Output owned"'; Expected = "command 'Invoke-Expression'" },
    [pscustomobject]@{ Label = "iex alias"; Source = 'iex "Write-Output owned"'; Expected = "command 'iex'" },
    [pscustomobject]@{ Label = "Invoke-Command"; Source = 'Invoke-Command -ScriptBlock { Write-Output owned }'; Expected = "command 'Invoke-Command'" },
    [pscustomobject]@{ Label = "Start-Process powershell"; Source = 'Start-Process -FilePath "powershell.exe"'; Expected = "Start-Process outside" },
    [pscustomobject]@{ Label = "Start-Process pwsh"; Source = 'Start-Process -FilePath "pwsh.exe"'; Expected = "Start-Process outside" },
    [pscustomobject]@{ Label = "Start-Process cmd"; Source = 'Start-Process -FilePath "cmd.exe"'; Expected = "Start-Process outside" },
    [pscustomobject]@{ Label = "System.IO File.Delete"; Source = 'if ($false) { [System.IO.File]::Delete("owned") }'; Expected = "path-addressed Delete" },
    [pscustomobject]@{ Label = "System.IO Directory.Delete"; Source = 'if ($false) { [System.IO.Directory]::Delete("owned") }'; Expected = "path-addressed Delete" },
    [pscustomobject]@{ Label = "System.IO File.Move"; Source = 'if ($false) { [System.IO.File]::Move("owned", "moved") }'; Expected = "path-addressed move or replace" },
    [pscustomobject]@{ Label = "System.IO File.Replace"; Source = 'if ($false) { [System.IO.File]::Replace("owned", "moved", "backup") }'; Expected = "path-addressed move or replace" },
    [pscustomobject]@{ Label = "FileInfo MoveTo"; Source = 'if ($false) { $file.MoveTo("moved") }'; Expected = "path-addressed MoveTo" },
    [pscustomobject]@{ Label = "generic Move-Item"; Source = 'if ($false) { Move-Item -LiteralPath "owned" -Destination "moved" }'; Expected = "Move-Item outside" },
    [pscustomobject]@{ Label = "forceful New-Item"; Source = 'if ($false) { New-Item -ItemType File -Path "owned" -Force }'; Expected = "forceful New-Item" },
    [pscustomobject]@{ Label = "unregistered Add-Type"; Source = 'Add-Type -TypeDefinition "public class Owned {}"'; Expected = "Add-Type outside" },
    [pscustomobject]@{ Label = "cmd del"; Source = 'if ($false) { cmd.exe /d /s /c "del /f /q owned" }'; Expected = "external executable" },
    [pscustomobject]@{ Label = "robocopy mirror"; Source = 'if ($false) { robocopy.exe source destination /MIR }'; Expected = "external executable" },
    [pscustomobject]@{ Label = "reflection InvokeMember"; Source = '$type.InvokeMember("Delete", 0, $null, $null, @())'; Expected = "reflection invocation" },
    [pscustomobject]@{ Label = "reflection DynamicInvoke"; Source = '$delegate.DynamicInvoke(@())'; Expected = "reflection invocation" },
    [pscustomobject]@{ Label = "reflection CreateDelegate"; Source = '[Delegate]::CreateDelegate([Action], $method)'; Expected = "reflection invocation" },
    [pscustomobject]@{ Label = "wrong module cmdlet qualifier"; Source = 'Unknown.Module\Write-Output "owned"'; Expected = "rejected module" }
)
foreach ($badFixture in $additionalDeletionAuditBadFixtures) {
    try {
        Assert-AmeR2cRDeletionSafetySource -SourceText ([string]$badFixture.Source) -Label ([string]$badFixture.Label)
        throw "R2c-R deletion safety audit accepted additional bad fixture '$($badFixture.Label)'"
    } catch {
        Assert-Contains $_.Exception.Message ([string]$badFixture.Expected)
    }
}

$payloadAuditFixture = $null
try {
    $payloadAuditFixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $toolPath -Nonce ([Guid]::NewGuid().ToString("N"))
    $payloadOutputRoot = [string]$payloadAuditFixture.Path
    $unsafeCommand = 'if ($false) { [System.IO.File]::Delete("owned") }'
    Assert-R2cRVerifiedPayloadRejected -Arguments @("-NoProfile", "-NonInteractive", "-Command", $unsafeCommand) -Expected "path-addressed Delete" -RepositoryRoot $repositoryRoot -OutputRoot $payloadOutputRoot
    $unsafeEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($unsafeCommand))
    Assert-R2cRVerifiedPayloadRejected -Arguments @("-NoProfile", "-NonInteractive", "-EncodedCommand", $unsafeEncoded) -Expected "path-addressed Delete" -RepositoryRoot $repositoryRoot -OutputRoot $payloadOutputRoot
    $nestedCommand = 'if ($false) {{ powershell.exe -NoProfile -EncodedCommand {0} }}' -f $unsafeEncoded
    Assert-R2cRVerifiedPayloadRejected -Arguments @("-NoProfile", "-NonInteractive", "-Command", $nestedCommand) -Expected "external executable" -RepositoryRoot $repositoryRoot -OutputRoot $payloadOutputRoot
    $nestedEncodedText = 'if ($false) { pwsh.exe -NoProfile -Command "Write-Output owned" }'
    $nestedEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($nestedEncodedText))
    Assert-R2cRVerifiedPayloadRejected -Arguments @("-NoProfile", "-NonInteractive", "-EncodedCommand", $nestedEncoded) -Expected "external executable" -RepositoryRoot $repositoryRoot -OutputRoot $payloadOutputRoot
} finally {
    if ($null -ne $payloadAuditFixture) {
        Remove-AmeR2cRDisposableRoot -Fixture $payloadAuditFixture
    }
}
$retiredManagedCleanupName = "Remove-AmeR2cRManaged" + "OwnedTree"
if ($commonText.IndexOf($retiredManagedCleanupName, [StringComparison]::Ordinal) -ge 0) {
    throw "R2c-R common cleanup retained a path-recursive fallback"
}
if ($runnerText -notmatch '(?s)try\s*\{.*SetEnvironmentVariable\("CARGO_BUILD_JOBS".*finally\s*\{') {
    throw "R2c-R runner must mutate build environment only inside its outer try/finally"
}
if ($runnerText -notmatch 'if \(\$null -ne \$toolLock\)') {
    throw "R2c-R runner must release only a successfully acquired tool lock"
}
if ($runnerText -notmatch 'if \(\$locationPushed\)') {
    throw "R2c-R runner must pop only a successfully pushed location"
}
if ($runnerText.IndexOf("Assert-AmeR2cRStrictDescendant", [StringComparison]::Ordinal) -ge 0) {
    throw "R2c-R runner must prove its root relationship by held identity, not path casing"
}

$hostValidation = if ($isCurrentExecutionContextSupported) { "passed" } else { "rejected" }
Write-Output (
    "AME_R2C_R_GUARDRAILS status=passed cases=19 " +
    "bootstrap=reflection-emit-held-active-race compiler_failure=fail-closed " +
    "cleanup=replacement-races-retained leaf=exact-ascii-hex-owned-only " +
    "tamper=executed-exact " +
    "anchor=workspace-held-handle production_anchor=known-folder-boundary " +
    "junction=internal-rejected script_identity=no-follow-held-revalidated " +
    "audit_budget=bounded-checked timeout=process-tree " +
    "platform_contract=windows11-x64 host_validation=$hostValidation"
)
