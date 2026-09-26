$ErrorActionPreference = "Stop"

$runner = Join-Path $PSScriptRoot "acceptance_run_windows_journal_broker.ps1"
$common = Join-Path $PSScriptRoot "acceptance_windows_journal_broker_common.ps1"
$releaseCommon = Join-Path $PSScriptRoot "release_journal_broker_common.ps1"
$integration = Join-Path $PSScriptRoot "integration_test_windows_journal_broker.ps1"
$client = Join-Path `
    (Split-Path -Parent $PSScriptRoot) `
    "rust\src\journal_broker\windows\acceptance.rs"
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ame-broker-acceptance-guardrail-$PID"
$workspace = Join-Path $testRoot "workspace"
$bundleV1 = Join-Path $testRoot "pre-signed-bundle-v1"
$bundleV2 = Join-Path $testRoot "pre-signed-bundle-v2"
$root = Join-Path $workspace "root"
$external = Join-Path $workspace "external"
$authorizationToken = "CEDARFLAKE_AME_WINDOWS_JOURNAL_BROKER_ACCEPTANCE_V1"

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value,
        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if ($Value.IndexOf($Expected, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Expected broker acceptance guardrail output to contain: $Expected"
    }
}

function Assert-Rejected {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Action,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    $rejected = $false
    try {
        & $Action
    } catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw $Message
    }
}

function Assert-PowerShellParses {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $tokens = $null
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile(
        $Path,
        [ref]$tokens,
        [ref]$errors
    )
    if ($errors.Count -ne 0) {
        throw "$Path failed to parse: $($errors.Message -join '; ')"
    }
}

Assert-PowerShellParses -Path $runner
Assert-PowerShellParses -Path $common
Assert-PowerShellParses -Path $integration
$runnerText = [System.IO.File]::ReadAllText($runner, [System.Text.Encoding]::UTF8)
$commonText = [System.IO.File]::ReadAllText($common, [System.Text.Encoding]::UTF8)
$integrationText = [System.IO.File]::ReadAllText($integration, [System.Text.Encoding]::UTF8)
$clientText = [System.IO.File]::ReadAllText($client, [System.Text.Encoding]::UTF8)
foreach ($required in @(
    "ValidationOnly",
    "AcknowledgeDisposableSystemChanges",
    "PreSignedBundleV1Path",
    "PreSignedBundleV2Path",
    "release_install_journal_broker.ps1",
    "release_repair_journal_broker.ps1",
    "release_upgrade_journal_broker.ps1",
    "release_test_journal_broker_installer_guardrails.ps1",
    "release_stop_journal_broker.ps1",
    "release_uninstall_journal_broker.ps1",
    "integration_test_windows_journal_broker.ps1",
    "New-ScheduledTaskPrincipal",
    "LogonType Interactive",
    "RunLevel Limited",
    "Stop-ScheduledTask",
    "Unregister-ScheduledTask",
    "DeletePhysicalEntry",
    "Set-AmeBrokerAcceptanceWorkspaceAcl",
    "Restore-AmeBrokerAcceptanceWorkspaceAcl"
)) {
    Assert-Contains -Value $runnerText -Expected $required
}
foreach ($required in @(
    "Assert-AmeBrokerPretrustedAcceptanceBundle",
    "Assert-AmeBrokerProtectedSourceFacts",
    "Assert-AmeBrokerAcceptanceBundlePairFacts",
    "Get-AmeBrokerNoFollowEntries",
    "Get-AmeBrokerFixtureState",
    "FILE_FLAG_OPEN_REPARSE_POINT",
    "SetFileInformationByHandle"
)) {
    Assert-Contains -Value $commonText -Expected $required
}
foreach ($requiredIntegration in @(
    "one_connected_client_does_not_consume_the_second_listener",
    "global_worker_budget_is_shared_and_reclaimed",
    "stop_signal_drains_eight_active_workers_without_detaching",
    "caller_pinned_mixed_name_semantics_cannot_be_downgraded_by_backend",
    "pinned_root_revalidation_rejects_rename_and_old_path_replacement",
    "reparse_attributes_are_always_rejected",
    "portable_production_never_invokes_the_journal_factory",
    "installed_production_defers_the_journal_factory_until_after_watcher_start",
    "unrelated_volume_storm_returns_bounded_strictly_advancing_pages"
)) {
    Assert-Contains -Value $integrationText -Expected $requiredIntegration
}
foreach ($forbidden in @(
    "New-SelfSignedCertificate",
    "CodeSigningCert",
    "Cert:\CurrentUser\Root",
    "Cert:\CurrentUser\TrustedPublisher",
    "Cert:\CurrentUser\My",
    "Set-AuthenticodeSignature",
    "DisposableBrokerSigningIdentity",
    "UseDisposableSelfSignedCertificate",
    "AcknowledgeTemporaryCurrentUserTrust"
)) {
    if ($runnerText.IndexOf($forbidden, [StringComparison]::Ordinal) -ge 0 -or
        $commonText.IndexOf($forbidden, [StringComparison]::Ordinal) -ge 0) {
        throw "The repository acceptance runner retained disposable signing capability: $forbidden"
    }
}
foreach ($required in @(
    "AME_BROKER_LIMITED_RESULT_V1",
    "register_root",
    "prove_protocol_mismatch",
    "assert_limited_token",
    "write_result"
)) {
    Assert-Contains -Value $clientText -Expected $required
}
if ($runnerText -match "Start-Process[^\r\n]+RunAs") {
    throw "The acceptance harness must never auto-elevate itself"
}
if ($runnerText -match '(?s)function Remove-AmeBrokerCreatedDirectory.*?Remove-Item') {
    throw "Acceptance cleanup regressed to a check-then-path delete"
}
if ($clientText -match "Start-Process[^\r\n]+RunAs") {
    throw "The limited acceptance client must never auto-elevate itself"
}
if (
    $runnerText -match "ResultPath|Export-Clixml|limited-client-.+\.txt" -or
    $runnerText -match "--ignored" -or
    $clientText -match "AME_JOURNAL_BROKER_ACCEPTANCE_LIMITED_TOKEN"
) {
    throw "The installed acceptance path retained a forgeable file or ignored-test bypass"
}
if ($commonText -match "Export-PfxCertificate|Export-Certificate|Import-Certificate|Password") {
    throw "The acceptance runner must not create, import, or export signing identities"
}
$validationIndex = $runnerText.IndexOf(
    'if ($ValidationOnly)',
    [System.StringComparison]::Ordinal
)
$administratorIndex = $runnerText.IndexOf(
    "Assert-AmeBrokerAdministrator",
    [System.StringComparison]::Ordinal
)
$bundleIndex = $runnerText.IndexOf(
    "Assert-AmeBrokerPretrustedAcceptanceBundle",
    [System.StringComparison]::Ordinal
)
$installIndex = $runnerText.IndexOf(
    'release_install_journal_broker.ps1',
    [System.StringComparison]::Ordinal
)
if ($bundleIndex -lt 0 -or $validationIndex -le $bundleIndex -or
    $administratorIndex -le $validationIndex -or $installIndex -le $validationIndex) {
    throw "ValidationOnly must validate the protected signed bundle before administrator or SCM paths"
}

New-Item -ItemType Directory -Path $workspace -Force | Out-Null
. $releaseCommon
. $common
$trustedAcl = [pscustomobject]@{
    OwnerSid = "S-1-5-32-544"
    Rules = @(
        [pscustomobject]@{
            Sid = "S-1-5-32-545"
            Rights = [uint64]0x00020089
            AccessControlType = "Allow"
            PropagationFlags = 0
        }
    )
}
$protectedFacts = @(
    [pscustomobject]@{
        ExpectedPath = "C:\protected"
        FinalPath = "C:\protected"
        IsReparsePoint = $false
        IsVolumeRoot = $false
        Acl = $trustedAcl
    },
    [pscustomobject]@{
        ExpectedPath = "C:\protected\bundle"
        FinalPath = "C:\protected\bundle"
        IsReparsePoint = $false
        IsVolumeRoot = $false
        Acl = $trustedAcl
    },
    [pscustomobject]@{
        ExpectedPath = "C:\protected\bundle\binary.exe"
        FinalPath = "C:\protected\bundle\binary.exe"
        IsReparsePoint = $false
        IsVolumeRoot = $false
        Acl = $trustedAcl
    }
)
Assert-AmeBrokerProtectedSourceFacts -Facts $protectedFacts
$unsafeAcl = [pscustomobject]@{
    OwnerSid = "S-1-5-32-544"
    Rules = @(
        [pscustomobject]@{
            Sid = "S-1-5-32-545"
            Rights = [uint64]0x00000002
            AccessControlType = "Allow"
            PropagationFlags = 0
        }
    )
}
$unsafeFacts = @($protectedFacts)
$unsafeFacts[2] = [pscustomobject]@{
    ExpectedPath = "C:\protected\bundle\binary.exe"
    FinalPath = "C:\protected\bundle\binary.exe"
    IsReparsePoint = $false
    IsVolumeRoot = $false
    Acl = $unsafeAcl
}
Assert-Rejected `
    -Action { Assert-AmeBrokerProtectedSourceFacts -Facts $unsafeFacts } `
    -Message "A user-writable pre-signed bundle unexpectedly passed admission"
$bundlePairV1 = [pscustomobject]@{
    BrokerSha256 = "11" * 32
    ClientSha256 = "33" * 32
    Protocol = 4
    MaximumFrameBytes = 1048576
}
$bundlePairV2 = [pscustomobject]@{
    BrokerSha256 = "22" * 32
    ClientSha256 = "33" * 32
    Protocol = 4
    MaximumFrameBytes = 1048576
}
Assert-AmeBrokerAcceptanceBundlePairFacts `
    -Version1 $bundlePairV1 `
    -Version2 $bundlePairV2
Assert-Rejected `
    -Action {
        Assert-AmeBrokerAcceptanceBundlePairFacts `
            -Version1 $bundlePairV1 `
            -Version2 ([pscustomobject]@{
                BrokerSha256 = "11" * 32
                ClientSha256 = "33" * 32
                Protocol = 4
                MaximumFrameBytes = 1048576
            })
    } `
    -Message "The acceptance pair admitted a no-op broker upgrade"
Assert-Rejected `
    -Action {
        Assert-AmeBrokerAcceptanceBundlePairFacts `
            -Version1 $bundlePairV1 `
            -Version2 ([pscustomobject]@{
                BrokerSha256 = "22" * 32
                ClientSha256 = "44" * 32
                Protocol = 4
                MaximumFrameBytes = 1048576
            })
    } `
    -Message "The broker-only upgrade admitted an uninstalled V2 client"
$handleCleanupRoot = Join-Path $testRoot "handle-cleanup"
$handleCleanupLeaf = Join-Path $handleCleanupRoot "fixture.bin"
New-Item -ItemType Directory -Path $handleCleanupRoot | Out-Null
[IO.File]::WriteAllBytes($handleCleanupLeaf, [byte[]](9, 8, 7, 6))
[Ame.JournalBrokerAcceptance.NativeMethods]::DeletePhysicalEntry(
    $handleCleanupLeaf,
    [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($handleCleanupLeaf),
    $false
)
[Ame.JournalBrokerAcceptance.NativeMethods]::DeletePhysicalEntry(
    $handleCleanupRoot,
    [Ame.JournalBrokerInstaller.NativeMethods]::GetFinalPath($handleCleanupRoot),
    $true
)
if (Test-Path -LiteralPath $handleCleanupRoot) {
    throw "The handle-anchored acceptance cleanup did not remove its exact fixture"
}
$nonce = "A1" * 32
$instance = "B2" * 16
$payload = "cancel=cancelled identity=00112233445566778899aabbccddeeff"
$payloadHex = ([BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($payload))).Replace("-", "").ToLowerInvariant()
$record = "AME_BROKER_LIMITED_RESULT_V1 nonce=$nonce phase=first " +
    "instance=$instance pid=42 status=passed payload_hex=$payloadHex"
$resultFacts = Assert-AmeBrokerLimitedResultFacts `
    -Record $record `
    -ExpectedNonce $nonce `
    -ExpectedPhase first `
    -ExpectedInstance $instance `
    -ExpectedProcessId 42
if ($resultFacts.Status -cne "passed" -or $resultFacts.Payload -cne $payload) {
    throw "The protected limited-client result parser lost bound evidence"
}
Assert-Rejected `
    -Action {
        Assert-AmeBrokerLimitedResultFacts `
            -Record $record `
            -ExpectedNonce ("C3" * 32) `
            -ExpectedPhase first `
            -ExpectedInstance $instance `
            -ExpectedProcessId 42
    } `
    -Message "The protected result parser accepted a replaced nonce"
Assert-AmeBrokerLimitedClientFacts `
    -ActualPath "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe" `
    -ExpectedPath "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe" `
    -IsElevated $false `
    -ActualSha256 ("D4" * 32) `
    -ExpectedSha256 ("D4" * 32) `
    -CreationTimeUtcFileTime 200 `
    -StartedAfterUtcFileTime 100
foreach ($replacement in @(
    [pscustomobject]@{
        Path = "C:\Users\Public\cedarflake_ame.exe"
        Elevated = $false
        Hash = "D4" * 32
        Created = 200
    },
    [pscustomobject]@{
        Path = "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe"
        Elevated = $true
        Hash = "D4" * 32
        Created = 200
    },
    [pscustomobject]@{
        Path = "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe"
        Elevated = $false
        Hash = "E5" * 32
        Created = 200
    },
    [pscustomobject]@{
        Path = "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe"
        Elevated = $false
        Hash = "D4" * 32
        Created = 99
    }
)) {
    Assert-Rejected `
        -Action {
            Assert-AmeBrokerLimitedClientFacts `
                -ActualPath $replacement.Path `
                -ExpectedPath "C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe" `
                -IsElevated $replacement.Elevated `
                -ActualSha256 $replacement.Hash `
                -ExpectedSha256 ("D4" * 32) `
                -CreationTimeUtcFileTime $replacement.Created `
                -StartedAfterUtcFileTime 100
        } `
        -Message "The protected result channel accepted a replacement client"
}
$taskBoundary = [DateTime]::UtcNow.AddSeconds(-1)
Assert-AmeBrokerTaskCompletionFacts `
    -LastTaskResult 0 `
    -LastRunTimeUtc ([DateTime]::UtcNow) `
    -StartedAfterUtc $taskBoundary
Assert-Rejected `
    -Action {
        Assert-AmeBrokerTaskCompletionFacts `
            -LastTaskResult 1 `
            -LastRunTimeUtc ([DateTime]::UtcNow) `
            -StartedAfterUtc $taskBoundary
    } `
    -Message "The task binding accepted a failed scheduler result"
$ancestorFacts = @(
    [pscustomobject]@{
        ExpectedPath = "C:\"
        FinalPath = "C:\"
        IsDirectory = $true
        IsReparsePoint = $false
    },
    [pscustomobject]@{
        ExpectedPath = "C:\Temp"
        FinalPath = "C:\Temp"
        IsDirectory = $true
        IsReparsePoint = $false
    }
)
Assert-AmeBrokerAcceptanceAncestorFacts -Facts $ancestorFacts
$ancestorFacts[1].IsReparsePoint = $true
Assert-Rejected `
    -Action { Assert-AmeBrokerAcceptanceAncestorFacts -Facts $ancestorFacts } `
    -Message "The workspace ancestor guard accepted a reparse point"
$ancestorFacts[1].IsReparsePoint = $false
$pipeInstance = New-AmeBrokerAcceptanceRandomHex -ByteCount 16
$pipeServer = New-AmeBrokerLimitedResultServer `
    -PipeName "CedarflakeAme.Acceptance.$pipeInstance" `
    -UserSid ([Security.Principal.WindowsIdentity]::GetCurrent().User)
try {
    $pipeSecurity = Get-AmeBrokerPipeSecurity -Pipe $pipeServer
    $pipeRules = @($pipeSecurity.GetAccessRules(
        $true,
        $false,
        [Security.Principal.SecurityIdentifier]
    ))
    if (-not $pipeSecurity.AreAccessRulesProtected) {
        throw "The protected result pipe DACL still permits inherited access rules"
    }
    $expectedPipeRules = @{
        "S-1-5-18" = [System.IO.Pipes.PipeAccessRights]::FullControl
        "S-1-5-32-544" = [System.IO.Pipes.PipeAccessRights]::FullControl
        $([Security.Principal.WindowsIdentity]::GetCurrent().User.Value) = `
            [System.IO.Pipes.PipeAccessRights]::ReadWrite -bor `
            [System.IO.Pipes.PipeAccessRights]::Synchronize
    }
    $observedPipeRules = @{}
    foreach ($rule in $pipeRules) {
        $sid = ([Security.Principal.SecurityIdentifier]$rule.IdentityReference).Value
        if ($rule.IsInherited -or
            $rule.AccessControlType -ne [Security.AccessControl.AccessControlType]::Allow -or
            -not $expectedPipeRules.ContainsKey($sid) -or
            $rule.PipeAccessRights -ne $expectedPipeRules[$sid] -or
            $observedPipeRules.ContainsKey($sid)) {
            throw "The protected result pipe DACL contains an unexpected access rule"
        }
        $observedPipeRules[$sid] = $true
    }
    if ($pipeRules.Count -ne $expectedPipeRules.Count -or
        $observedPipeRules.Count -ne $expectedPipeRules.Count) {
        throw "The protected result pipe DACL is not the exact three-principal allowlist"
    }
} finally {
    $pipeServer.Dispose()
}
$snapshotRoot = Join-Path $testRoot "snapshot"
New-Item -ItemType Directory -Path $snapshotRoot | Out-Null
$snapshotFile = Join-Path $snapshotRoot "fixture.bin"
[System.IO.File]::WriteAllBytes($snapshotFile, [byte[]](1, 2, 3, 4))
$snapshotBefore = Get-AmeBrokerFixtureState -Path $snapshotRoot
if ((Get-AmeBrokerFixtureState -Path $snapshotRoot) -cne $snapshotBefore) {
    throw "Fixture snapshot must be stable on Windows PowerShell 5.1"
}
[System.IO.File]::WriteAllBytes($snapshotFile, [byte[]](4, 3, 2, 1))
if ((Get-AmeBrokerFixtureState -Path $snapshotRoot) -ceq $snapshotBefore) {
    throw "Fixture snapshot failed to detect source content mutation"
}
$signedFixtureSource = Join-Path $env:SystemRoot "System32\where.exe"
$signedFixtureSignature = Get-AuthenticodeSignature -LiteralPath $signedFixtureSource
if ($signedFixtureSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $signedFixtureSignature.SignerCertificate) {
    throw "The broker acceptance guardrail requires a valid signed Windows x64 system binary"
}
$publisher = $signedFixtureSignature.SignerCertificate.Subject
$signedFixtureV2Source = Join-Path $env:SystemRoot "System32\whoami.exe"
$signedFixtureV2Signature = Get-AuthenticodeSignature -LiteralPath $signedFixtureV2Source
if ($signedFixtureV2Signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $signedFixtureV2Signature.SignerCertificate -or
    $signedFixtureV2Signature.SignerCertificate.Subject -cne $publisher) {
    throw "The broker acceptance guardrail requires two same-publisher signed x64 binaries"
}
foreach ($bundle in @($bundleV1, $bundleV2)) {
    New-Item -ItemType Directory -Path (Join-Path $bundle "Application") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $bundle "Broker") -Force | Out-Null
    Copy-Item `
        -LiteralPath $signedFixtureSource `
        -Destination (Join-Path $bundle "Application\cedarflake_ame.exe")
}
Copy-Item `
    -LiteralPath $signedFixtureSource `
    -Destination (Join-Path $bundleV1 "Broker\cedarflake_ame_journal_broker.exe")
Copy-Item `
    -LiteralPath $signedFixtureV2Source `
    -Destination (Join-Path $bundleV2 "Broker\cedarflake_ame_journal_broker.exe")

try {
    try {
        & $runner `
            -PreSignedBundleV1Path $bundleV1 `
            -PreSignedBundleV2Path $bundleV2 `
            -ExpectedPublisher $publisher `
            -AcceptanceWorkspace $workspace `
            -DisposableRoot $root `
            -ExternalSiblingRoot $external `
            -AuthorizationToken "wrong" `
            -ValidationOnly
        throw "Invalid broker acceptance authorization unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "exact current Windows journal broker acceptance token"
    }

    $outside = Join-Path $testRoot "outside-root"
    try {
        & $runner `
            -PreSignedBundleV1Path $bundleV1 `
            -PreSignedBundleV2Path $bundleV2 `
            -ExpectedPublisher $publisher `
            -AcceptanceWorkspace $workspace `
            -DisposableRoot $outside `
            -ExternalSiblingRoot $external `
            -AuthorizationToken $authorizationToken `
            -ValidationOnly
        throw "Out-of-workspace disposable root unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "direct workspace child"
    }

    Assert-Rejected `
        -Action {
            Assert-AmeSignedX64Binary `
                -Path $signedFixtureSource `
                -ExpectedPublisher "CN=Wrong Publisher" | Out-Null
        } `
        -Message "A wrong signed publisher unexpectedly passed admission"
    Assert-Rejected `
        -Action {
            & $runner `
                -PreSignedBundleV1Path $bundleV1 `
                -PreSignedBundleV2Path $bundleV2 `
                -ExpectedPublisher $publisher `
                -AcceptanceWorkspace $workspace `
                -DisposableRoot $root `
                -ExternalSiblingRoot $external `
                -AuthorizationToken $authorizationToken `
                -ValidationOnly
        } `
        -Message "A user-writable signed bundle unexpectedly passed ValidationOnly"
    if (@(Get-ChildItem -LiteralPath $workspace -Force).Count -ne 0) {
        throw "ValidationOnly mutated the disposable acceptance workspace"
    }

    Write-Output "AME_BROKER_ACCEPTANCE_GUARDRAILS status=passed"
} finally {
    if (Test-Path -LiteralPath $testRoot -PathType Container) {
        Remove-AmeBrokerOwnedTreeNoFollow `
            -Path $testRoot `
            -ExpectedParent ([System.IO.Path]::GetDirectoryName($testRoot))
    }
    if (Test-Path -LiteralPath $testRoot) {
        throw "The acceptance guardrail scratch tree still exists after no-follow cleanup"
    }
}
