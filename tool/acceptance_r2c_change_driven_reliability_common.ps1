. (Join-Path $PSScriptRoot "acceptance_r2c_cargo_invocation.ps1")

$script:AmeR2cRForbiddenEnvironmentNames = @(
    "CEDARFLAKE_AME_R2C_R_CATALOG",
    "CEDARFLAKE_AME_R2C_R_CURRENT_PATH",
    "CEDARFLAKE_AME_R2C_R_EXPECTED_SIZE",
    "CEDARFLAKE_AME_R2C_R_NEXT_USN",
    "CEDARFLAKE_AME_R2C_R_OPERATION",
    "CEDARFLAKE_AME_R2C_R_OWNED_ROOT",
    "CEDARFLAKE_AME_R2C_R_PARENT_PID",
    "CEDARFLAKE_AME_R2C_R_PREVIEWS",
    "CEDARFLAKE_AME_R2C_R_PREVIOUS_PATH",
    "CEDARFLAKE_AME_R2C_R_REPORT",
    "CEDARFLAKE_AME_R2C_R_ROOT_ID",
    "CEDARFLAKE_AME_R2C_R_ROOT_PATH",
    "CEDARFLAKE_AME_R2C_R_RUNNER_PID",
    "CEDARFLAKE_AME_R2C_R_RUN_NONCE",
    "CEDARFLAKE_AME_R2C_R_SETTINGS",
    "CEDARFLAKE_AME_R2C_R_SOURCE_ROOT",
    "CEDARFLAKE_AME_R2C_R_START_USN",
    "CEDARFLAKE_AME_R2C_R_WORKER_PHASE"
)

$script:AmeR2cRCaseDefinitions = @(
    [pscustomobject]@{ Label = "production-coordinator-smoke"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_controlled_journal_evidence_uses_the_production_coordinator"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "live-operations-event-storm"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_live_operations_and_event_storm_remain_p0_bounded"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $true },
    [pscustomobject]@{ Label = "closed-process-durable-p1"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_closed_process_changes_converge_through_durable_p1"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $true },
    [pscustomobject]@{ Label = "watcher-overflow-active-p0"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_watcher_overflow_recovers_while_twenty_five_real_p0_changes_remain_fast"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $true },
    [pscustomobject]@{ Label = "million-unrelated-backlog"; FullyQualifiedName = "journal_broker::windows::usn::tests::million_unrelated_records_stream_through_bounded_production_parser_pages"; SourcePath = "rust\src\journal_broker\windows\usn.rs"; Ignored = $true },
    [pscustomobject]@{ Label = "no-change-one-hundred"; FullyQualifiedName = "application::library_synchronization::production::tests::startup::one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning"; SourcePath = "rust\src\application\library_synchronization\production\tests\startup.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "priority-p0-p1-p2"; FullyQualifiedName = "application::library_synchronization::production::tests::priority::p0_event_to_visible_p95_stays_below_one_second_with_p1_and_p2_active"; SourcePath = "rust\src\application\library_synchronization\production\tests\priority.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "journal-reset-trim"; FullyQualifiedName = "application::persistent_journal_continuity::session_reader::tests::session_reader_classifies_reset_and_trim_from_structured_query_fields"; SourcePath = "rust\src\application\persistent_journal_continuity\session_reader\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-reset-trim-race"; FullyQualifiedName = "journal_broker::service::tests::reset_and_trimming_during_read_fail_closed"; SourcePath = "rust\src\journal_broker\service\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-runtime-restart"; FullyQualifiedName = "application::library_synchronization::production::runtime_lifecycle::tests::journal_sessions::journal_factory_reopens_only_after_the_previous_runtime_retires"; SourcePath = "rust\src\application\library_synchronization\production\runtime_lifecycle\tests\journal_sessions.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-close-failure-blocks-restart"; FullyQualifiedName = "application::library_synchronization::production::runtime_lifecycle::tests::journal_sessions::failed_journal_close_prevents_a_replacement_session"; SourcePath = "rust\src\application\library_synchronization\production\runtime_lifecycle\tests\journal_sessions.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "shutdown-restart"; FullyQualifiedName = "application::library_synchronization::production::runtime_lifecycle::tests::shutdown::one_stop_joins_delayed_p0_p1_p2_then_allows_immediate_restart"; SourcePath = "rust\src\application\library_synchronization\production\runtime_lifecycle\tests\shutdown.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "cancellation-blocked-io"; FullyQualifiedName = "application::library_synchronization::production::runtime_lifecycle::tests::shutdown::stop_and_restart_remain_responsive_during_blocked_filesystem_and_writer_work"; SourcePath = "rust\src\application\library_synchronization\production\runtime_lifecycle\tests\shutdown.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "root-replacement"; FullyQualifiedName = "application::incremental_library_changes::tests::replacement_root_uses_only_the_proof_guard_and_enumerates_nothing"; SourcePath = "rust\src\application\incremental_library_changes\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "same-volume-multi-root"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_same_volume_roots_share_one_retained_session_read_and_isolate_failure"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "placeholder-zero-media"; FullyQualifiedName = "application::metadata_inventory::tests::placeholder_evidence_is_staged_and_enqueued_without_media_inspection"; SourcePath = "rust\src\application\metadata_inventory\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "placeholder-no-hydration"; FullyQualifiedName = "application::metadata_inventory::tests::unchanged_reparse_cloud_placeholder_preserves_location_without_retry"; SourcePath = "rust\src\application\metadata_inventory\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "chinese-long-path"; FullyQualifiedName = "application::directory_synchronization::tests::chinese_and_long_relative_paths_remain_lossless_and_normalized"; SourcePath = "rust\src\application\directory_synchronization\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "worker-report-binding-tamper"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_worker_report_fields_reject_missing_or_ambiguous_bindings"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $false }
)

$script:AmeR2cRCompilerFaultPointForGuardrail = $null
$script:AmeR2cRCompilerFaultOwnerForGuardrail = $null
$script:AmeR2cRVerifiedProcessBoundaryDigest = "5a038d883b362986055110431b4e212db60aae223868b798423bc0bf5a5d6202"
$script:AmeR2cRNativeProcessBoundaryDigest = "4e58cf525fd2f628a38bfefb4b1728fbf6e66632b3c03220d67a2249c3e55860"
$script:AmeR2cRAuditedScriptSnapshotDigest = "165355dcd5dedbf01a73953547f1b71a7f9503ff3b02ed0d8c9eb15e7e54a0cf"
$script:AmeR2cRNativeDefinitionDigest = "07e3aaaa4d257669682568a10c9bbb4fb3230c479d501102c6f8d7fe22312426"
$script:AmeR2cRGuardrailMoveSourceDigest = "2082dbca806e1fc914c4877809c82a670515d7065c377380d0fae6586908eaa8"
$script:AmeR2cRDeletionAuditBudgetLimits = [ordered]@{
    "source-count" = [uint64]9
    "dot-source-depth" = [uint64]8
    "per-source-bytes" = [uint64](256 * 1024)
    "total-bytes" = [uint64](512 * 1024)
    "ast-nodes" = [uint64]32768
    "function-count" = [uint64]128
    "scope-count" = [uint64]512
    "queue-count" = [uint64]512
}
$script:AmeR2cRAllowedCmdlets = @{
    "Add-Type" = "Microsoft.PowerShell.Utility"
    "ConvertFrom-Json" = "Microsoft.PowerShell.Utility"
    "ConvertTo-Json" = "Microsoft.PowerShell.Utility"
    "ForEach-Object" = "Microsoft.PowerShell.Core"
    "Get-Command" = "Microsoft.PowerShell.Core"
    "Get-Content" = "Microsoft.PowerShell.Management"
    "Get-Process" = "Microsoft.PowerShell.Management"
    "Join-Path" = "Microsoft.PowerShell.Management"
    "Move-Item" = "Microsoft.PowerShell.Management"
    "New-Item" = "Microsoft.PowerShell.Management"
    "Out-Null" = "Microsoft.PowerShell.Core"
    "Out-String" = "Microsoft.PowerShell.Utility"
    "Pop-Location" = "Microsoft.PowerShell.Management"
    "Push-Location" = "Microsoft.PowerShell.Management"
    "Select-Object" = "Microsoft.PowerShell.Utility"
    "Set-StrictMode" = "Microsoft.PowerShell.Core"
    "Sort-Object" = "Microsoft.PowerShell.Utility"
    "Split-Path" = "Microsoft.PowerShell.Management"
    "Start-Process" = "Microsoft.PowerShell.Management"
    "Start-Sleep" = "Microsoft.PowerShell.Utility"
    "Stop-Process" = "Microsoft.PowerShell.Management"
    "Test-Path" = "Microsoft.PowerShell.Management"
    "Where-Object" = "Microsoft.PowerShell.Core"
    "Write-Host" = "Microsoft.PowerShell.Utility"
    "Write-Output" = "Microsoft.PowerShell.Utility"
}

. (Join-Path $PSScriptRoot "acceptance_r2c_compiler_bootstrap.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_native_resources.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_process_execution.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_source_audit.ps1")

function Get-AmeR2cRForbiddenEnvironmentNames { return @($script:AmeR2cRForbiddenEnvironmentNames) }
function Get-AmeR2cRCaseDefinitions { return @($script:AmeR2cRCaseDefinitions) }

function Test-AmeR2cRCurrentProcessIsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    try {
        $principal = [Security.Principal.WindowsPrincipal]::new($identity)
        return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    } finally { $identity.Dispose() }
}

function Get-AmeR2cRWindowsVersionEvidence {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64
    )
    try {
        $currentVersion = $base.OpenSubKey("SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        $productOptions = $base.OpenSubKey("SYSTEM\CurrentControlSet\Control\ProductOptions")
        if ($null -eq $currentVersion -or $null -eq $productOptions) {
            throw "R2c-R could not read trusted Windows version evidence"
        }
        try {
            $build = [int]$currentVersion.GetValue("CurrentBuildNumber", "0")
            $displayVersion = [string]$currentVersion.GetValue("DisplayVersion", "unknown")
            $installationType = [string]$currentVersion.GetValue("InstallationType", "")
            $productType = [string]$productOptions.GetValue("ProductType", "")
        } finally {
            $currentVersion.Dispose()
            $productOptions.Dispose()
        }
    } finally { $base.Dispose() }
    $apiVersion = [AmeR2cRNative]::NativeVersion()
    return [pscustomobject]@{
        IsWindows = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([Runtime.InteropServices.OSPlatform]::Windows)
        BuildNumber = $build
        ApiBuildNumber = $apiVersion.Build
        DisplayVersion = $displayVersion
        InstallationType = $installationType
        ProductType = $productType
        ProductSku = [AmeR2cRNative]::ProductSku([uint32]$apiVersion.Major, [uint32]$apiVersion.Minor)
    }
}

function Assert-AmeR2cRExecutionContext {
    param(
        [Parameter(Mandatory = $true)][bool]$IsWindowsPlatform,
        [Parameter(Mandatory = $true)][System.Runtime.InteropServices.Architecture]$OperatingSystemArchitecture,
        [Parameter(Mandatory = $true)][System.Runtime.InteropServices.Architecture]$ProcessArchitecture,
        [Parameter(Mandatory = $true)][int]$BuildNumber,
        [Parameter(Mandatory = $true)][int]$ApiBuildNumber,
        [Parameter(Mandatory = $true)][string]$InstallationType,
        [Parameter(Mandatory = $true)][string]$ProductType,
        [Parameter(Mandatory = $true)][uint32]$ProductSku,
        [Parameter(Mandatory = $true)][bool]$IsAdministrator
    )
    if (-not $IsWindowsPlatform) { throw "R2c-R change-driven reliability requires Windows" }
    if ($OperatingSystemArchitecture -ne [Runtime.InteropServices.Architecture]::X64 -or $ProcessArchitecture -ne [Runtime.InteropServices.Architecture]::X64) { throw "R2c-R change-driven reliability requires an x64 process on Windows x64" }
    if ($BuildNumber -lt 22000 -or $ApiBuildNumber -lt 22000) { throw "R2c-R change-driven reliability requires Windows 11 build 22000 or later" }
    if ($BuildNumber -ne $ApiBuildNumber) { throw "R2c-R Windows build evidence disagrees between registry and version API" }
    if ($InstallationType -cne "Client" -or $ProductType -cne "WinNT") { throw "R2c-R change-driven reliability requires a Windows 11 client workstation SKU" }
    if ($ProductSku -eq 0) { throw "R2c-R change-driven reliability requires a resolved Windows client SKU" }
    if ($IsAdministrator) { throw "R2c-R local reliability must run as an ordinary non-administrator user" }
}

function Assert-AmeR2cRNoReparseAncestors {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.DirectoryInfo]::new($Path)
    while ($null -ne $current) {
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "R2c-R disposable storage contains a reparse-point ancestor" }
        $current = $current.Parent
    }
}

function New-AmeR2cRDisposableRoot {
    param([Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-f]{32}$')][string]$Nonce)
    if ($null -eq ("AmeR2cRDisposableRoot" -as [type])) {
        throw "R2c-R native types must be initialized before creating disposable storage"
    }
    return [AmeR2cRDisposableRoot]::Create($Nonce)
}

function New-AmeR2cRGuardrailDisposableRoot {
    param(
        [Parameter(Mandatory = $true)][string]$AnchorPath,
        [Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-f]{32}$')][string]$Nonce
    )
    if ($null -eq ("AmeR2cRDisposableRoot" -as [type])) {
        throw "R2c-R native types must be initialized before creating guardrail storage"
    }
    return [AmeR2cRDisposableRoot]::CreateForGuardrail($AnchorPath, $Nonce)
}

. (Join-Path $PSScriptRoot "acceptance_r2c_fixture_cleanup.ps1")

function Remove-AmeR2cRDisposableRoot {
    param(
        [Parameter(Mandatory = $true)][psobject]$Fixture,
        [switch]$RetainFixtureOnFailureForGuardrail,
        [switch]$ReleaseRootBlockerOnlyForGuardrail,
        [switch]$DeleteReleasedRootForGuardrail
    )
    Invoke-AmeR2cRDisposableRootCleanup @PSBoundParameters
}

function Get-AmeR2cRCaseArguments {
    param([Parameter(Mandatory = $true)][psobject]$Case)
    $arguments = @("test", "--locked", "--manifest-path", "rust\Cargo.toml", "--lib", [string]$Case.FullyQualifiedName, "--", "--exact", "--nocapture", "--test-threads=1")
    if ([bool]$Case.Ignored) { $arguments += "--ignored" }
    return $arguments
}

function Assert-AmeR2cRCaseMatrix {
    param([Parameter(Mandatory = $true)][string]$RepositoryRoot)
    $cases = @(Get-AmeR2cRCaseDefinitions)
    if ($cases.Count -ne 19) { throw "R2c-R case matrix must contain exactly 19 cases" }
    if (@($cases | Where-Object { $_.Ignored }).Count -ne 4) { throw "R2c-R case matrix must contain exactly four ignored manual cases" }
    if (@($cases | Where-Object { -not $_.Ignored }).Count -ne 15) { throw "R2c-R case matrix must contain exactly fifteen normal cases" }
    if (@($cases.Label | Sort-Object -Unique).Count -ne 19) { throw "R2c-R case labels must be unique" }
    if (@($cases.FullyQualifiedName | Sort-Object -Unique).Count -ne 19) { throw "R2c-R fully qualified test names must be unique" }
    $sameVolume = @($cases | Where-Object { $_.Label -ceq "same-volume-multi-root" })
    $expectedSameVolume = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_same_volume_roots_share_one_retained_session_read_and_isolate_failure"
    if ($sameVolume.Count -ne 1 -or $sameVolume[0].FullyQualifiedName -cne $expectedSameVolume) { throw "R2c-R same-volume case must target the retained production coordinator path" }
    foreach ($case in $cases) {
        $arguments = @(Get-AmeR2cRCaseArguments -Case $case)
        if (@($arguments | Where-Object { $_ -ceq "--exact" }).Count -ne 1) { throw "R2c-R case '$($case.Label)' must use exactly one --exact selector" }
        if ((@($arguments | Where-Object { $_ -ceq "--ignored" }).Count -eq 1) -ne [bool]$case.Ignored) { throw "R2c-R case '$($case.Label)' has an inconsistent ignored selector" }
        $source = Join-Path $RepositoryRoot ([string]$case.SourcePath)
        if (-not [IO.File]::Exists($source)) { throw "R2c-R case '$($case.Label)' source file is missing" }
        $functionName = ([string]$case.FullyQualifiedName).Split(':')[-1]
        $sourceText = [IO.File]::ReadAllText($source)
        $mapping = [regex]::Match(
            $sourceText,
            (
                "(?ms)(?<attributes>(?:^\s*#\[[^\r\n]+\]\s*\r?\n)+)" +
                "^\s*fn\s+" + [regex]::Escape($functionName) + "\s*\("
            )
        )
        if (-not $mapping.Success) { throw "R2c-R case '$($case.Label)' does not map to a current attributed test function" }
        $attributes = $mapping.Groups["attributes"].Value
        if ($attributes -notmatch '(?m)^\s*#\[test\]\s*$') { throw "R2c-R case '$($case.Label)' is not a test" }
        $isActuallyIgnored = $attributes -match '(?m)^\s*#\[ignore(?:\s*=.*)?\]\s*$'
        if ($isActuallyIgnored -ne [bool]$case.Ignored) { throw "R2c-R case '$($case.Label)' declared ignored state does not match its test attribute" }
    }
}

function Assert-AmeR2cRCaseResult {
    param([Parameter(Mandatory = $true)][string]$Label, [Parameter(Mandatory = $true)][AllowEmptyString()][string[]]$Lines, [Parameter(Mandatory = $true)][int]$ExitCode)
    if ($ExitCode -ne 0) { throw "R2c-R case '$Label' failed with exit code $ExitCode" }
    $resultLines = @($Lines | Where-Object { $_ -match '^test result: ' })
    if ($resultLines.Count -eq 0) { throw "R2c-R case '$Label' produced no test result" }
    $result = [regex]::Match($resultLines[$resultLines.Count - 1], '^test result: ok\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;.*$')
    if (-not $result.Success) { throw "R2c-R case '$Label' produced an invalid result summary" }
    if ([int]$result.Groups[1].Value -ne 1 -or [int]$result.Groups[2].Value -ne 0 -or
        [int]$result.Groups[3].Value -ne 0 -or [int]$result.Groups[4].Value -ne 0) {
        throw "R2c-R case '$Label' did not execute exactly one passing, non-filtered-only test: $($result.Value)"
    }
}
