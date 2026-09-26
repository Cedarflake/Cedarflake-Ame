function Assert-AmeR2cRFixtureLifetime {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "R2c-R fixture lifetime regression: $Message" }
}

function Invoke-AmeR2cRFixtureLifetimeTests {
    param([Parameter(Mandatory = $true)][string]$ToolPath)

    foreach ($kind in @("ordinary", "junction")) {
        $fixture = $null
        $target = $null
        $metadataHandle = $null
        $identity = $null
        $retired = $false
        $leaf = "lifetime-$([Guid]::NewGuid().ToString('N'))"
        $cleanup = New-AmeR2cRFixtureCleanup
        $primaryFailure = $null
        try {
            $fixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath `
                -Nonce ([Guid]::NewGuid().ToString("N"))
            $path = Join-Path $fixture.Path $leaf
            if ($kind -ceq "junction") {
                $target = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath `
                    -Nonce ([Guid]::NewGuid().ToString("N"))
                New-Item -ItemType Junction -Path $path -Target $target.Path | Out-Null
                $identity = $fixture.CaptureGuardrailChildDirectoryIdentity($leaf, $true)
                $reparseFailure = $null
                try { $fixture.DeleteGuardrailChildDirectoryForCleanup($leaf, $identity, $false) } catch { $reparseFailure = $_ }
                Assert-AmeR2cRFixtureLifetime ($null -ne $reparseFailure -and $reparseFailure.Exception.Message.Contains("reparse point")) "junction escaped the expected-kind check"
                $target.ValidateHeldIdentity()
            } else {
                $identity = $fixture.CreateGuardrailChildDirectory($leaf)
            }
            $metadataHandle = [AmeR2cRBootstrapNative]::CreateFileW(
                $path, 0, 3, [IntPtr]::Zero, 3, 0x02200000, [IntPtr]::Zero
            )
            Assert-AmeR2cRFixtureLifetime (-not $metadataHandle.IsInvalid) "metadata holder failed"
            $fixture.DeleteGuardrailChildDirectoryForCleanup($leaf, $identity, ($kind -ceq "junction"))
            $retired = $true
            $entries = @([IO.DirectoryInfo]::new($fixture.Path).EnumerateFileSystemInfos())
            Assert-AmeR2cRFixtureLifetime ($entries.Count -eq 0) "$kind deletion returned before namespace retirement"
            Assert-AmeR2cRFixtureLifetime (-not $metadataHandle.IsClosed) "regression retired the observation handle before asserting absence"
            if ($null -ne $target) { $target.ValidateHeldIdentity() }
        } catch {
            $primaryFailure = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_
        } finally {
            $children = @()
            if ($null -ne $identity -and -not $retired) {
                $children = @([pscustomobject]@{ Leaf = $leaf; Identity = $identity; IsReparse = ($kind -ceq "junction") })
            }
            Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label $kind -Fixture $fixture `
                -Handle $metadataHandle -Children $children
            Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label "$kind-target" -Fixture $target
        }
        Complete-AmeR2cRFixtureCleanup -Cleanup $cleanup -PrimaryFailure $primaryFailure
    }

    foreach ($case in @("data-holder", "wrong-identity", "nonempty")) {
        $fixture = $null
        $holder = $null
        $sentinel = $null
        $identity = $null
        $leaf = "guard-$([Guid]::NewGuid().ToString('N'))"
        $cleanup = New-AmeR2cRFixtureCleanup
        $primaryFailure = $null
        try {
            $fixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath `
                -Nonce ([Guid]::NewGuid().ToString("N"))
            $identity = $fixture.CreateGuardrailChildDirectory($leaf)
            $path = Join-Path $fixture.Path $leaf
            $expectedIdentity = $identity
            if ($case -ceq "data-holder") {
                $holder = [AmeR2cRBootstrapNative]::CreateFileW(
                    $path, 1, 3, [IntPtr]::Zero, 3, 0x02200000, [IntPtr]::Zero
                )
                Assert-AmeR2cRFixtureLifetime (-not $holder.IsInvalid) "directory-data holder failed"
            } elseif ($case -ceq "wrong-identity") {
                $expectedIdentity = "00000000:00000000:00000000"
            } else {
                $sentinel = [IO.FileStream]::new(
                    (Join-Path $path "sentinel.bin"), [IO.FileMode]::CreateNew,
                    [IO.FileAccess]::ReadWrite, ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
                    4096, [IO.FileOptions]::DeleteOnClose
                )
                $sentinel.WriteByte(42)
                $sentinel.Flush($true)
            }
            $failure = $null
            try { $fixture.DeleteGuardrailChildDirectoryForCleanup($leaf, $expectedIdentity, $false) } catch { $failure = $_ }
            Assert-AmeR2cRFixtureLifetime ($null -ne $failure) "$case lost its deletion refusal"
            if ($case -ceq "wrong-identity") {
                Assert-AmeR2cRFixtureLifetime ($failure.Exception.Message.Contains("identity changed")) "identity refusal was replaced by an unrelated failure"
            } else {
                $nativeFailure = $failure.Exception.GetBaseException()
                $expectedError = if ($case -ceq "data-holder") { 32 } else { 145 }
                Assert-AmeR2cRFixtureLifetime ($nativeFailure -is [ComponentModel.Win32Exception] -and $nativeFailure.NativeErrorCode -eq $expectedError) "$case returned the wrong native refusal"
            }
            Assert-AmeR2cRFixtureLifetime ([IO.Directory]::Exists($path)) "$case removed a protected directory"
            if ($null -ne $sentinel) {
                $sentinel.Position = 0
                Assert-AmeR2cRFixtureLifetime ($sentinel.ReadByte() -eq 42) "nonempty refusal changed sentinel content"
            }
        } catch {
            $primaryFailure = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_
        } finally {
            $children = @()
            if ($null -ne $identity) {
                $children = @([pscustomobject]@{ Leaf = $leaf; Identity = $identity; IsReparse = $false })
            }
            Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label $case -Fixture $fixture `
                -Handle $holder -Stream $sentinel -Children $children
        }
        Complete-AmeR2cRFixtureCleanup -Cleanup $cleanup -PrimaryFailure $primaryFailure
    }
    Invoke-AmeR2cRFixtureCleanupFailureTest -ToolPath $ToolPath
    Invoke-AmeR2cRFixtureInventoryBoundTest -ToolPath $ToolPath
    Write-Output "r2c_r_fixture_namespace_retirement_passed"
}

function Invoke-AmeR2cRFixtureInventoryBoundTest {
    param([Parameter(Mandatory = $true)][string]$ToolPath)

    $fixture = $null
    $children = [Collections.Generic.Dictionary[string, string]]::new()
    $cleanup = New-AmeR2cRFixtureCleanup
    $primaryFailure = $null
    try {
        $fixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath -Nonce ([Guid]::NewGuid().ToString("N"))
        for ($index = 0; $index -lt 65; $index++) {
            $leaf = "bounded-$index-$([Guid]::NewGuid().ToString('N'))"
            $children.Add($leaf, $fixture.CreateGuardrailChildDirectory($leaf))
        }
        $caught = $null
        try { Remove-AmeR2cRDisposableRoot -Fixture $fixture -RetainFixtureOnFailureForGuardrail } catch { $caught = $_.Exception }
        Assert-AmeR2cRFixtureLifetime ($null -ne $caught) "bounded observation deleted a nonempty root"
        $retained = $caught.Data["ameR2cRRetainedRoot"]
        Assert-AmeR2cRFixtureLifetime ($retained.observedEntries.Count -eq 64 -and $retained.entriesTruncated) "failure inventory exceeded its metadata bound or lost truncation"
        foreach ($entry in $retained.observedEntries) {
            Assert-AmeR2cRFixtureLifetime ($children.ContainsKey($entry.name) -and $children[$entry.name] -ceq $entry.identity) "bounded observation lost the physical child identity"
        }
        $retiredLeaf = @($children.Keys)[0]
        $retiredEntry = [IO.DirectoryInfo]::new((Join-Path $fixture.Path $retiredLeaf))
        $retiredEntry.Refresh()
        $fixture.DeleteGuardrailChildDirectoryForCleanup($retiredLeaf, $children[$retiredLeaf], $false)
        $null = $children.Remove($retiredLeaf)
        $missingObservation = @(Get-AmeR2cRRemainingFixtureEntries -Fixture $fixture -Entries @($retiredEntry))
        Assert-AmeR2cRFixtureLifetime ($missingObservation.Count -eq 1 -and $null -ne $missingObservation[0].observationFailure) "a vanished entry escaped the auxiliary observation boundary"
    } catch {
        $primaryFailure = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_
    } finally {
        $remaining = @($children.GetEnumerator() | ForEach-Object {
            [pscustomobject]@{ Leaf = $_.Key; Identity = $_.Value; IsReparse = $false }
        })
        Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label "inventory-bound" `
            -Fixture $fixture -Children $remaining
    }
    Complete-AmeR2cRFixtureCleanup -Cleanup $cleanup -PrimaryFailure $primaryFailure
}

function Invoke-AmeR2cRFixtureCleanupFailureTest {
    param([Parameter(Mandatory = $true)][string]$ToolPath)

    $fixture = $null
    $sentinelFixture = $null
    $sentinel = $null
    $firstIdentity = $null
    $secondIdentity = $null
    $secondRetired = $false
    $firstLeaf = "first-$([Guid]::NewGuid().ToString('N'))"
    $secondLeaf = "second-$([Guid]::NewGuid().ToString('N'))"
    $caseCleanup = New-AmeR2cRFixtureCleanup
    $primaryFailure = $null
    try {
        $fixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath -Nonce ([Guid]::NewGuid().ToString("N"))
        $firstIdentity = $fixture.CreateGuardrailChildDirectory($firstLeaf)
        $secondIdentity = $fixture.CreateGuardrailChildDirectory($secondLeaf)
        $sentinelFixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $ToolPath -Nonce ([Guid]::NewGuid().ToString("N"))
        $sentinel = [IO.FileStream]::new(
            (Join-Path $sentinelFixture.Path "sentinel.bin"), [IO.FileMode]::CreateNew,
            [IO.FileAccess]::ReadWrite, ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete),
            4096, [IO.FileOptions]::DeleteOnClose
        )
        $cleanup = New-AmeR2cRFixtureCleanup
        $original = [InvalidOperationException]::new("original controlled fixture failure")
        try { throw $original } catch { $original = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_ }
        Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label "failed" -Fixture $fixture `
            -RetainFixtureOnFailureForGuardrail -Children @(
                [pscustomobject]@{ Leaf = $firstLeaf; Identity = "00000000:00000000:00000000"; IsReparse = $false }
                [pscustomobject]@{ Leaf = $secondLeaf; Identity = $secondIdentity; IsReparse = $false }
            )
        $secondRetired = -not [IO.Directory]::Exists((Join-Path $fixture.Path $secondLeaf))
        Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label "sentinel" `
            -Fixture $sentinelFixture -Stream $sentinel
        Assert-AmeR2cRFixtureLifetime $secondRetired "a child failure skipped the next owned child"
        Assert-AmeR2cRFixtureLifetime (-not $sentinel.CanRead -and -not $sentinelFixture.IsRootHandleHeld) "a root failure skipped the independent sentinel retirement"
        Assert-AmeR2cRFixtureLifetime ($cleanup.Failures.Count -eq 2) "child and nonempty-root failures were not both preserved"
        $retained = $cleanup.Failures[1].exception.Data["ameR2cRRetainedRoot"]
        Assert-AmeR2cRFixtureLifetime ($retained.observedEntries.Count -eq 1 -and $retained.observedEntries[0].identity -ceq $firstIdentity) "failure lost the immediate remaining child identity"
        $caught = $null
        $transcript = [Collections.Generic.List[string]]::new()
        try {
            Complete-AmeR2cRFixtureCleanup -Cleanup $cleanup -PrimaryFailure $original |
                ForEach-Object { $transcript.Add([string]$_) }
        } catch { $caught = $_.Exception }
        Assert-AmeR2cRFixtureLifetime ([object]::ReferenceEquals($caught, $original)) "cleanup replaced the original fixture exception"
        Assert-AmeR2cRFixtureLifetime ($caught.Data["ameR2cRFixtureRetirements"].Count -eq 3) "failure lost owned handle retirement evidence"
        Assert-AmeR2cRFixtureLifetime ($transcript.Count -eq 1 -and $transcript[0].Contains($firstIdentity)) "retained identity was not emitted before failure"
        $record = $transcript[0].Substring("AME_R2C_R_FIXTURE_CLEANUP ".Length) | ConvertFrom-Json
        Assert-AmeR2cRFixtureLifetime ($record.originalErrorText -ceq "original controlled fixture failure" -and
            -not [string]::IsNullOrEmpty($record.originalScriptStack) -and
            $record.originalScriptStack -ceq $original.Data["ameR2cROriginalScriptStack"]) "failure lost its original PowerShell error or stack"
    } catch {
        $primaryFailure = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_
    } finally {
        $remaining = @()
        if ($null -ne $firstIdentity) {
            $remaining += [pscustomobject]@{ Leaf = $firstLeaf; Identity = $firstIdentity; IsReparse = $false }
        }
        if ($null -ne $secondIdentity -and -not $secondRetired) {
            $remaining += [pscustomobject]@{ Leaf = $secondLeaf; Identity = $secondIdentity; IsReparse = $false }
        }
        Close-AmeR2cRGuardrailFixture -Cleanup $caseCleanup -Label "failure-case" `
            -Fixture $fixture -Children $remaining
        $heldSentinel = $null
        if ($null -ne $sentinelFixture -and $sentinelFixture.IsRootHandleHeld) {
            $heldSentinel = $sentinelFixture
        }
        Close-AmeR2cRGuardrailFixture -Cleanup $caseCleanup -Label "failure-case-sentinel" `
            -Fixture $heldSentinel -Stream $sentinel
    }
    Complete-AmeR2cRFixtureCleanup -Cleanup $caseCleanup -PrimaryFailure $primaryFailure
}
