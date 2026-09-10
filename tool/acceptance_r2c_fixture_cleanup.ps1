function Invoke-AmeR2cRDisposableRootCleanup {
    param(
        [Parameter(Mandatory = $true)][psobject]$Fixture,
        [switch]$RetainFixtureOnFailureForGuardrail,
        [switch]$ReleaseRootBlockerOnlyForGuardrail,
        [switch]$DeleteReleasedRootForGuardrail
    )
    if (($ReleaseRootBlockerOnlyForGuardrail -or $DeleteReleasedRootForGuardrail) -and
        -not $RetainFixtureOnFailureForGuardrail) {
        throw "R2c-R cleanup race phases are available only to retained internal guardrails"
    }
    if ($ReleaseRootBlockerOnlyForGuardrail -and $DeleteReleasedRootForGuardrail) {
        throw "R2c-R cleanup race phases must be selected one at a time"
    }
    $cleanupFailure = $null
    $disposalFailure = $null
    $rootPath = [string]$Fixture.Path
    $rootIdentity = [string]$Fixture.RootIdentityToken
    $remainingEntries = @()
    $entriesTruncated = $false
    $shouldDispose = -not $ReleaseRootBlockerOnlyForGuardrail
    try {
        if ([IO.Path]::GetFileName($rootPath) -cne [string]$Fixture.RootName -or
            [string]$Fixture.RootName -notmatch '^Cedarflake-Ame-R2c-R-[0-9a-f]{32}-[0-9a-f]{32}$') {
            throw "R2c-R refused to clean a disposable root with the wrong owned name"
        }
        if ($DeleteReleasedRootForGuardrail) {
            $Fixture.DeleteReleasedRootForCleanup()
        } else {
            $Fixture.ValidateHeldIdentity()
            $entries = [Collections.Generic.List[IO.FileSystemInfo]]::new()
            $enumerator = [IO.DirectoryInfo]::new($rootPath).EnumerateFileSystemInfos().GetEnumerator()
            try {
                while ($entries.Count -lt 64 -and $enumerator.MoveNext()) {
                    $entries.Add($enumerator.Current)
                }
                if ($entries.Count -eq 64) { $entriesTruncated = $enumerator.MoveNext() }
            } finally { $enumerator.Dispose() }
            if ($entries.Count -ne 0) {
                $remainingEntries = @(Get-AmeR2cRRemainingFixtureEntries -Fixture $Fixture -Entries $entries.ToArray())
                throw (
                    "R2c-R safe cleanup found a non-empty owned root and retained it " +
                    "without traversing unknown children"
                )
            }
            if ($ReleaseRootBlockerOnlyForGuardrail) {
                $Fixture.ReleaseRootHandleForCleanup()
            } else {
                $Fixture.DeleteEmptyRootForCleanup()
            }
        }
    } catch {
        $cleanupFailure = $_
    } finally {
        if ($shouldDispose -and
            ($null -eq $cleanupFailure -or -not $RetainFixtureOnFailureForGuardrail)) {
            try { $Fixture.Dispose() } catch {
                $disposalFailure = $_
                if ($null -eq $cleanupFailure) { $cleanupFailure = $_ }
            }
        }
    }
    if ($null -ne $cleanupFailure) {
        $failure = [InvalidOperationException]::new((
            "R2c-R safe cleanup failed and retained the owned root for investigation; " +
            "no path fallback was attempted; owned_leaf=$($Fixture.RootName); " +
            "expected_identity=$rootIdentity; original_path=$rootPath; " +
            $cleanupFailure.Exception.Message
        ), $cleanupFailure.Exception)
        $failure.Data["ameR2cRRetainedRoot"] = [ordered]@{
            path = $rootPath; expectedIdentity = $rootIdentity
            observedEntries = $remainingEntries; entriesTruncated = $entriesTruncated
            rootHandleHeld = $Fixture.IsRootHandleHeld
            originalError = $cleanupFailure.ToString(); originalStack = $cleanupFailure.ScriptStackTrace
            disposalFailure = $(if ($null -ne $disposalFailure) { $disposalFailure.Exception.ToString() } else { $null })
        }
        throw $failure
    }
}

function Get-AmeR2cRRemainingFixtureEntries {
    param([psobject]$Fixture, [IO.FileSystemInfo[]]$Entries)

    foreach ($entry in $Entries) {
        $handle = $null
        $record = [ordered]@{ name = $entry.Name; attributes = $null; identity = $null; observationFailure = $null }
        try {
            $record.attributes = $entry.Attributes.ToString()
            $Fixture.ValidateHeldIdentity()
            $handle = [AmeR2cRBootstrapNative]::CreateFileW(
                (Join-Path $Fixture.Path $entry.Name), 0, 3, [IntPtr]::Zero, 3, 0x02300000, [IntPtr]::Zero
            )
            if ($handle.IsInvalid) { throw "Remaining fixture entry could not be opened without following reparses" }
            $information = [Activator]::CreateInstance(("AmeR2cRBootstrapFileInformation" -as [type]))
            if (-not [AmeR2cRBootstrapNative]::GetFileInformationByHandle($handle, [ref]$information)) {
                throw "Remaining fixture entry identity could not be read"
            }
            $record.identity = "{0:X8}:{1:X8}:{2:X8}" -f (
                [uint32]$information.VolumeSerialNumber, [uint32]$information.FileIndexHigh, [uint32]$information.FileIndexLow
            )
            $Fixture.ValidateHeldIdentity()
        } catch { $record.observationFailure = $_.Exception.Message } finally {
            if ($null -ne $handle) { $handle.Dispose() }
        }
        $record
    }
}

function New-AmeR2cRFixtureCleanup {
    [pscustomobject]@{
        PSTypeName = "Ame.R2cR.FixtureCleanup"
        Failures = [Collections.Generic.List[object]]::new()
        Retirements = [Collections.Generic.List[object]]::new()
    }
}

function Get-AmeR2cRFixturePrimaryFailure {
    param([Parameter(Mandatory = $true)][Management.Automation.ErrorRecord]$ErrorRecord)

    $failure = $ErrorRecord.Exception
    if (-not $failure.Data.Contains("ameR2cROriginalErrorText")) {
        $failure.Data["ameR2cROriginalErrorText"] = $ErrorRecord.ToString()
        $failure.Data["ameR2cROriginalScriptStack"] = $ErrorRecord.ScriptStackTrace
    }
    return $failure
}

function Close-AmeR2cRGuardrailFixture {
    param(
        [Parameter(Mandatory = $true)][psobject]$Cleanup,
        [Parameter(Mandatory = $true)][string]$Label,
        [AllowNull()][psobject]$Fixture,
        [object[]]$Children = @(),
        [AllowNull()][IO.Stream]$Stream,
        [AllowNull()][Runtime.InteropServices.SafeHandle]$Handle,
        [switch]$RetainFixtureOnFailureForGuardrail
    )

    if ($null -ne $Handle) {
        try { $Handle.Dispose() } catch {
            $Cleanup.Failures.Add([pscustomobject]@{ stage = "$Label-handle"; exception = $_.Exception })
        }
        $Cleanup.Retirements.Add([ordered]@{ stage = "$Label-handle"; closed = $Handle.IsClosed })
    }
    if ($null -ne $Stream) {
        try { $Stream.Dispose() } catch {
            $Cleanup.Failures.Add([pscustomobject]@{ stage = "$Label-stream"; exception = $_.Exception })
        }
        $Cleanup.Retirements.Add([ordered]@{ stage = "$Label-stream"; closed = -not $Stream.CanRead })
    }
    if ($null -eq $Fixture) { return }
    foreach ($child in $Children) {
        try {
            $Fixture.DeleteGuardrailChildDirectoryForCleanup(
                [string]$child.Leaf, [string]$child.Identity, [bool]$child.IsReparse
            )
        } catch {
            $Cleanup.Failures.Add([pscustomobject]@{ stage = "$Label-child"; leaf = $child.Leaf; expectedIdentity = $child.Identity; exception = $_.Exception })
        }
    }
    try {
        Remove-AmeR2cRDisposableRoot -Fixture $Fixture `
            -RetainFixtureOnFailureForGuardrail:$RetainFixtureOnFailureForGuardrail
    } catch {
        $Cleanup.Failures.Add([pscustomobject]@{ stage = "$Label-root"; exception = $_.Exception })
    }
    $Cleanup.Retirements.Add([ordered]@{ stage = "$Label-root"; closed = -not $Fixture.IsRootHandleHeld })
}

function Complete-AmeR2cRFixtureCleanup {
    param([psobject]$Cleanup, [AllowNull()][Exception]$PrimaryFailure)

    if ($null -eq $PrimaryFailure -and $Cleanup.Failures.Count -ne 0) {
        $PrimaryFailure = $Cleanup.Failures[0].exception
    }
    if ($null -ne $PrimaryFailure) {
        $PrimaryFailure.Data["ameR2cRFixtureCleanupFailures"] = @($Cleanup.Failures.ToArray())
        $PrimaryFailure.Data["ameR2cRFixtureRetirements"] = @($Cleanup.Retirements.ToArray())
        $record = [ordered]@{
            format = "ame-r2c-r-fixture-cleanup-v1"
            originalFailure = $PrimaryFailure.ToString()
            originalErrorText = $PrimaryFailure.Data["ameR2cROriginalErrorText"]
            originalScriptStack = $PrimaryFailure.Data["ameR2cROriginalScriptStack"]
            failures = @($Cleanup.Failures | ForEach-Object {
                [ordered]@{
                    stage = $_.stage; message = $_.exception.ToString()
                    retainedRoot = $_.exception.Data["ameR2cRRetainedRoot"]
                }
            })
            retirements = @($Cleanup.Retirements.ToArray())
        }
        Write-Output "AME_R2C_R_FIXTURE_CLEANUP $($record | Microsoft.PowerShell.Utility\ConvertTo-Json -Depth 8 -Compress)"
        throw $PrimaryFailure
    }
}
