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
    [pscustomobject]@{ Label = "no-change-one-hundred"; FullyQualifiedName = "application::library_synchronization::production::tests::one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "priority-p0-p1-p2"; FullyQualifiedName = "application::library_synchronization::production::tests::p0_event_to_visible_p95_stays_below_one_second_with_p1_and_p2_active"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "journal-reset-trim"; FullyQualifiedName = "application::persistent_journal_continuity::session_reader::tests::session_reader_classifies_reset_and_trim_from_structured_query_fields"; SourcePath = "rust\src\application\persistent_journal_continuity\session_reader\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-reset-trim-race"; FullyQualifiedName = "journal_broker::service::tests::reset_and_trimming_during_read_fail_closed"; SourcePath = "rust\src\journal_broker\service\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-reconnect"; FullyQualifiedName = "application::library_synchronization::production::tests::production_journal_factory_reconnects_after_closing_the_previous_session"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "broker-reconnect-close-failure"; FullyQualifiedName = "application::library_synchronization::production::tests::reconnect_isolates_a_close_failure_and_installs_a_fresh_session"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "shutdown-restart"; FullyQualifiedName = "application::library_synchronization::production::tests::one_stop_joins_delayed_p0_p1_p2_then_allows_immediate_restart"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "cancellation-blocked-io"; FullyQualifiedName = "application::library_synchronization::production::tests::stop_and_restart_remain_responsive_during_blocked_filesystem_and_writer_work"; SourcePath = "rust\src\application\library_synchronization\production.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "root-replacement"; FullyQualifiedName = "application::incremental_library_changes::tests::replacement_root_uses_only_the_proof_guard_and_enumerates_nothing"; SourcePath = "rust\src\application\incremental_library_changes\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "same-volume-multi-root"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_same_volume_roots_share_one_retained_session_read_and_isolate_failure"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "placeholder-zero-media"; FullyQualifiedName = "application::metadata_inventory::tests::placeholder_evidence_is_staged_and_enqueued_without_media_inspection"; SourcePath = "rust\src\application\metadata_inventory\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "placeholder-no-hydration"; FullyQualifiedName = "application::metadata_inventory::tests::unchanged_reparse_cloud_placeholder_preserves_location_without_retry"; SourcePath = "rust\src\application\metadata_inventory\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "chinese-long-path"; FullyQualifiedName = "application::directory_synchronization::tests::chinese_and_long_relative_paths_remain_lossless_and_normalized"; SourcePath = "rust\src\application\directory_synchronization\tests.rs"; Ignored = $false },
    [pscustomobject]@{ Label = "worker-report-binding-tamper"; FullyQualifiedName = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_worker_report_fields_reject_missing_or_ambiguous_bindings"; SourcePath = "rust\test_support\r2c_r_change_driven_reliability_acceptance.rs"; Ignored = $false }
)

$script:AmeR2cRCompilerFaultPointForGuardrail = $null
$script:AmeR2cRCompilerFaultOwnerForGuardrail = $null
$script:AmeR2cRVerifiedProcessBoundaryDigest = "c4702c1005217d99cb2d0558c7de0c341d98ab8f3e33160677065da7f266b6e7"
$script:AmeR2cRNativeProcessBoundaryDigest = "4e58cf525fd2f628a38bfefb4b1728fbf6e66632b3c03220d67a2249c3e55860"
$script:AmeR2cRAuditedScriptSnapshotDigest = "165355dcd5dedbf01a73953547f1b71a7f9503ff3b02ed0d8c9eb15e7e54a0cf"
$script:AmeR2cRNativeDefinitionDigest = "2da77861b8b5fb1a65ab474d8e9f22406fa3c049ec771d88288587bb7e579259"
$script:AmeR2cRGuardrailMoveSourceDigest = "ee64f5de764073fa63e6edb5c5c9c1f0e44f4e6bf889e016e27423f805cee141"
$script:AmeR2cRDeletionAuditBudgetLimits = [ordered]@{
    "source-count" = [uint64]8
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

function Start-AmeR2cRCompilerFaultForGuardrail {
    param([Parameter(Mandatory = $true)][string]$Point)
    if ($null -ne $script:AmeR2cRCompilerFaultPointForGuardrail -or
        $null -ne $script:AmeR2cRCompilerFaultOwnerForGuardrail) {
        throw "R2c-R compiler fault seam was not clean"
    }
    $script:AmeR2cRCompilerFaultPointForGuardrail = $Point
}

function Stop-AmeR2cRCompilerFaultForGuardrail {
    $script:AmeR2cRCompilerFaultPointForGuardrail = $null
}

function Invoke-AmeR2cRCompilerFaultForGuardrail {
    param(
        [Parameter(Mandatory = $true)][string]$Point,
        [Parameter(Mandatory = $true)][object]$Owner
    )
    if ($script:AmeR2cRCompilerFaultPointForGuardrail -cne $Point) { return }
    $script:AmeR2cRCompilerFaultOwnerForGuardrail = $Owner
    throw "R2c-R injected compiler ownership fault: $Point"
}

function Test-AmeR2cRCompilerFaultOwnerClosedForGuardrail {
    $owner = $script:AmeR2cRCompilerFaultOwnerForGuardrail
    if ($null -eq $owner) { return $false }
    if ($owner -is [Microsoft.Win32.SafeHandles.SafeFileHandle]) {
        return $owner.IsClosed
    }
    if ($null -ne $owner.RootHandle -and -not $owner.RootHandle.IsClosed) { return $false }
    foreach ($handle in @($owner.ToolAnchorHandles)) {
        if ($null -ne $handle -and -not $handle.IsClosed) { return $false }
    }
    return $true
}

function Reset-AmeR2cRCompilerFaultOwnerForGuardrail {
    $owner = $script:AmeR2cRCompilerFaultOwnerForGuardrail
    $script:AmeR2cRCompilerFaultOwnerForGuardrail = $null
    if ($owner -is [Microsoft.Win32.SafeHandles.SafeFileHandle] -and -not $owner.IsClosed) {
        $owner.Dispose()
    }
}

function Assert-AmeR2cRManagedNoReparseAncestors {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.DirectoryInfo]::new([IO.Path]::GetFullPath($Path))
    while ($null -ne $current) {
        if (-not $current.Exists) {
            throw "R2c-R managed trust anchor does not exist"
        }
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "R2c-R managed trust anchor contains a reparse-point ancestor"
        }
        $current = $current.Parent
    }
}

function Complete-AmeR2cREmittedType {
    param([Parameter(Mandatory = $true)][Reflection.Emit.TypeBuilder]$TypeBuilder)
    $createType = $TypeBuilder.GetType().GetMethod("CreateType", [Type[]]@())
    if ($null -ne $createType) { return $TypeBuilder.CreateType() }
    return $TypeBuilder.CreateTypeInfo().AsType()
}

function Add-AmeR2cREmittedStruct {
    param(
        [Parameter(Mandatory = $true)][Reflection.Emit.ModuleBuilder]$ModuleBuilder,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][object[]]$Fields
    )
    $attributes = [Reflection.TypeAttributes]::Public -bor
        [Reflection.TypeAttributes]::Sealed -bor
        [Reflection.TypeAttributes]::SequentialLayout -bor
        [Reflection.TypeAttributes]::BeforeFieldInit
    $builder = $ModuleBuilder.DefineType($Name, $attributes, [ValueType])
    foreach ($field in $Fields) {
        $builder.DefineField(
            [string]$field.Name,
            [type]$field.Type,
            [Reflection.FieldAttributes]::Public
        ) | Out-Null
    }
    return Complete-AmeR2cREmittedType -TypeBuilder $builder
}

function Add-AmeR2cREmittedPInvoke {
    param(
        [Parameter(Mandatory = $true)][Reflection.Emit.TypeBuilder]$TypeBuilder,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Library,
        [Parameter(Mandatory = $true)][type]$ReturnType,
        [Parameter(Mandatory = $true)][type[]]$ParameterTypes,
        [Parameter(Mandatory = $true)][Runtime.InteropServices.CharSet]$CharacterSet
    )
    $attributes = [Reflection.MethodAttributes]::Public -bor
        [Reflection.MethodAttributes]::Static -bor
        [Reflection.MethodAttributes]::PinvokeImpl
    $method = $TypeBuilder.DefinePInvokeMethod(
        $Name,
        $Library,
        $attributes,
        [Reflection.CallingConventions]::Standard,
        $ReturnType,
        $ParameterTypes,
        [Runtime.InteropServices.CallingConvention]::Winapi,
        $CharacterSet
    )
    $method.SetImplementationFlags(
        $method.GetMethodImplementationFlags() -bor
            [Reflection.MethodImplAttributes]::PreserveSig
    )
}

function Initialize-AmeR2cRBootstrapNativeTypes {
    if ($null -ne ("AmeR2cRBootstrapNative" -as [type])) { return }

    $assemblyName = [Reflection.AssemblyName]::new(
        "AmeR2cRBootstrapNative-$([Guid]::NewGuid().ToString('N'))"
    )
    try {
        $assembly = [Reflection.Emit.AssemblyBuilder]::DefineDynamicAssembly(
            $assemblyName,
            [Reflection.Emit.AssemblyBuilderAccess]::Run
        )
    } catch [Management.Automation.MethodException] {
        $assembly = [AppDomain]::CurrentDomain.DefineDynamicAssembly(
            $assemblyName,
            [Reflection.Emit.AssemblyBuilderAccess]::Run
        )
    }
    $module = $assembly.DefineDynamicModule($assemblyName.Name)
    $fileInformation = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapFileInformation" -Fields @(
            [pscustomobject]@{ Name = "FileAttributes"; Type = [uint32] },
            [pscustomobject]@{ Name = "CreationTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "CreationTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastAccessTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastAccessTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastWriteTimeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "LastWriteTimeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "VolumeSerialNumber"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileSizeHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileSizeLow"; Type = [uint32] },
            [pscustomobject]@{ Name = "NumberOfLinks"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileIndexHigh"; Type = [uint32] },
            [pscustomobject]@{ Name = "FileIndexLow"; Type = [uint32] }
        )
    $unicodeString = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapUnicodeString" -Fields @(
            [pscustomobject]@{ Name = "Length"; Type = [uint16] },
            [pscustomobject]@{ Name = "MaximumLength"; Type = [uint16] },
            [pscustomobject]@{ Name = "Buffer"; Type = [IntPtr] }
        )
    $objectAttributes = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapObjectAttributes" -Fields @(
            [pscustomobject]@{ Name = "Length"; Type = [uint32] },
            [pscustomobject]@{ Name = "RootDirectory"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "ObjectName"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "Attributes"; Type = [uint32] },
            [pscustomobject]@{ Name = "SecurityDescriptor"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "SecurityQualityOfService"; Type = [IntPtr] }
        )
    $ioStatusBlock = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapIoStatusBlock" -Fields @(
            [pscustomobject]@{ Name = "Status"; Type = [IntPtr] },
            [pscustomobject]@{ Name = "Information"; Type = [UIntPtr] }
        )
    $disposition = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapDisposition" -Fields @(
            [pscustomobject]@{ Name = "DeleteFile"; Type = [int32] }
        )
    $caseSensitiveInformation = Add-AmeR2cREmittedStruct -ModuleBuilder $module `
        -Name "AmeR2cRBootstrapCaseSensitiveInformation" -Fields @(
            [pscustomobject]@{ Name = "Flags"; Type = [uint32] }
        )

    $builder = $module.DefineType(
        "AmeR2cRBootstrapNative",
        [Reflection.TypeAttributes]::Public -bor
            [Reflection.TypeAttributes]::Sealed -bor
            [Reflection.TypeAttributes]::Abstract
    )
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "CreateFileW" `
        -Library "kernel32.dll" `
        -ReturnType ([Microsoft.Win32.SafeHandles.SafeFileHandle]) `
        -ParameterTypes @(
            [string], [uint32], [uint32], [IntPtr], [uint32], [uint32], [IntPtr]
        ) -CharacterSet Unicode
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFileInformationByHandle" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            $fileInformation.MakeByRefType()
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFileInformationByHandleEx" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [uint32],
            $caseSensitiveInformation.MakeByRefType(),
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "GetFinalPathNameByHandleW" `
        -Library "kernel32.dll" -ReturnType ([uint32]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [Text.StringBuilder],
            [uint32],
            [uint32]
        ) -CharacterSet Unicode
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "SetFileInformationByHandle" `
        -Library "kernel32.dll" -ReturnType ([bool]) `
        -ParameterTypes @(
            [Microsoft.Win32.SafeHandles.SafeFileHandle],
            [uint32],
            $disposition.MakeByRefType(),
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "NtCreateFile" `
        -Library "ntdll.dll" -ReturnType ([int32]) `
        -ParameterTypes @(
            [IntPtr].MakeByRefType(),
            [uint32],
            $objectAttributes.MakeByRefType(),
            $ioStatusBlock.MakeByRefType(),
            [IntPtr],
            [uint32],
            [uint32],
            [uint32],
            [uint32],
            [IntPtr],
            [uint32]
        ) -CharacterSet None
    Add-AmeR2cREmittedPInvoke -TypeBuilder $builder -Name "RtlNtStatusToDosError" `
        -Library "ntdll.dll" -ReturnType ([uint32]) `
        -ParameterTypes @([int32]) -CharacterSet None
    Complete-AmeR2cREmittedType -TypeBuilder $builder | Out-Null
}

function Get-AmeR2cRBootstrapHandleEvidence {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$Handle,
        [Parameter(Mandatory = $true)][string]$Label,
        [switch]$AllowReparse
    )
    if ($Handle.IsInvalid -or $Handle.IsClosed) {
        throw "R2c-R $Label handle is not held"
    }
    $informationType = "AmeR2cRBootstrapFileInformation" -as [type]
    $information = [Activator]::CreateInstance($informationType)
    if (-not [AmeR2cRBootstrapNative]::GetFileInformationByHandle(
        $Handle,
        [ref]$information
    )) {
        throw "R2c-R could not inspect the $Label handle"
    }
    if (($information.FileAttributes -band [uint32]0x10) -eq 0) {
        throw "R2c-R $Label is not a directory"
    }
    $isReparse = ($information.FileAttributes -band [uint32]0x400) -ne 0
    if ($isReparse -and -not $AllowReparse) {
        throw "R2c-R $Label is a reparse point"
    }
    $buffer = [Text.StringBuilder]::new(32768)
    $length = [AmeR2cRBootstrapNative]::GetFinalPathNameByHandleW(
        $Handle,
        $buffer,
        [uint32]$buffer.Capacity,
        0
    )
    if ($length -eq 0 -or $length -ge $buffer.Capacity) {
        throw "R2c-R could not resolve the $Label handle"
    }
    $resolved = $buffer.ToString()
    if ($resolved.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
        $resolved = '\\' + $resolved.Substring(8)
    } elseif ($resolved.StartsWith('\\?\', [StringComparison]::OrdinalIgnoreCase)) {
        $resolved = $resolved.Substring(4)
    }
    return [pscustomobject]@{
        Attributes = [uint32]$information.FileAttributes
        Identity = "{0:X8}:{1:X8}:{2:X8}" -f (
            [uint32]$information.VolumeSerialNumber,
            [uint32]$information.FileIndexHigh,
            [uint32]$information.FileIndexLow
        )
        IsReparse = $isReparse
        Path = $resolved.TrimEnd('\')
        VolumeSerialNumber = [uint32]$information.VolumeSerialNumber
    }
}

function Test-AmeR2cRBootstrapDirectoryCaseSensitive {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$Handle
    )
    $informationType = "AmeR2cRBootstrapCaseSensitiveInformation" -as [type]
    $information = [Activator]::CreateInstance($informationType)
    if (-not [AmeR2cRBootstrapNative]::GetFileInformationByHandleEx(
        $Handle,
        [uint32]23,
        [ref]$information,
        [uint32][Runtime.InteropServices.Marshal]::SizeOf($information)
    )) {
        throw "R2c-R could not query compiler-anchor case semantics"
    }
    return ([uint32]$information.Flags -band [uint32]1) -ne 0
}

function Open-AmeR2cRBootstrapRelativeDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$ParentHandle,
        [Parameter(Mandatory = $true)][string]$LeafName,
        [Parameter(Mandatory = $true)][bool]$Create,
        [Parameter(Mandatory = $true)][bool]$RequestDelete,
        [switch]$RequestWriteAttributes,
        [switch]$ExistingComponent
    )
    if ($ExistingComponent) {
        if ([string]::IsNullOrEmpty($LeafName) -or $LeafName -in @(".", "..") -or
            $LeafName.IndexOfAny([char[]]@('\', '/', ':', [char]0)) -ge 0 -or
            $LeafName.EndsWith('.', [StringComparison]::Ordinal) -or
            $LeafName.EndsWith(' ', [StringComparison]::Ordinal)) {
            throw "R2c-R compiler anchor component is not one normalized NT name"
        }
    } elseif ($LeafName -notmatch '^(?:\.ame-r2c-r-bootstrap-[0-9a-f]{32}|\.ame-r2c-r-audit-[0-9a-f]{32}\.ps1)$') {
        throw "R2c-R compiler or audited-script guardrail leaf is outside the owned-name contract"
    }
    $unicodeType = "AmeR2cRBootstrapUnicodeString" -as [type]
    $attributesType = "AmeR2cRBootstrapObjectAttributes" -as [type]
    $statusType = "AmeR2cRBootstrapIoStatusBlock" -as [type]
    $nameBuffer = [Runtime.InteropServices.Marshal]::StringToHGlobalUni($LeafName)
    $nameStructure = [IntPtr]::Zero
    $rawHandle = [IntPtr]::Zero
    try {
        $unicodeName = [Activator]::CreateInstance($unicodeType)
        $unicodeName.Length = [uint16]($LeafName.Length * 2)
        $unicodeName.MaximumLength = [uint16](($LeafName.Length + 1) * 2)
        $unicodeName.Buffer = $nameBuffer
        $nameStructure = [Runtime.InteropServices.Marshal]::AllocHGlobal(
            [Runtime.InteropServices.Marshal]::SizeOf($unicodeName)
        )
        [Runtime.InteropServices.Marshal]::StructureToPtr(
            $unicodeName,
            $nameStructure,
            $false
        )
        $attributes = [Activator]::CreateInstance($attributesType)
        $attributes.Length = [uint32][Runtime.InteropServices.Marshal]::SizeOf($attributes)
        $attributes.RootDirectory = $ParentHandle.DangerousGetHandle()
        $attributes.ObjectName = $nameStructure
        $attributes.Attributes = [uint32]0
        if (-not (Test-AmeR2cRBootstrapDirectoryCaseSensitive -Handle $ParentHandle)) {
            $attributes.Attributes = [uint32]($attributes.Attributes -bor 0x40)
        }
        $statusBlock = [Activator]::CreateInstance($statusType)
        $desiredAccess = [uint32](0x80 -bor 0x100000)
        if (-not $ExistingComponent) {
            $desiredAccess = [uint32]($desiredAccess -bor 0x1)
        }
        if ($RequestDelete) { $desiredAccess = [uint32]($desiredAccess -bor 0x10000) }
        if ($RequestWriteAttributes) { $desiredAccess = [uint32]($desiredAccess -bor 0x100) }
        $status = [AmeR2cRBootstrapNative]::NtCreateFile(
            [ref]$rawHandle,
            $desiredAccess,
            [ref]$attributes,
            [ref]$statusBlock,
            [IntPtr]::Zero,
            [uint32]0x10,
            [uint32](0x1 -bor 0x2),
            [uint32]$(if ($Create) { 2 } else { 1 }),
            [uint32](0x1 -bor 0x20 -bor 0x4000 -bor 0x200000),
            [IntPtr]::Zero,
            0
        )
        if ($status -lt 0) {
            $win32 = [AmeR2cRBootstrapNative]::RtlNtStatusToDosError($status)
            $operation = if ($Create) { "create" } else { "reopen" }
            throw (
                "R2c-R could not $operation relative directory '$LeafName' from its held " +
                "parent; NTSTATUS=0x$($status.ToString('X8')) Win32=$win32"
            )
        }
        $handle = [Microsoft.Win32.SafeHandles.SafeFileHandle]::new($rawHandle, $true)
        $rawHandle = [IntPtr]::Zero
        return $handle
    } finally {
        if ($rawHandle -ne [IntPtr]::Zero -and $rawHandle -ne [IntPtr](-1)) {
            [Microsoft.Win32.SafeHandles.SafeFileHandle]::new($rawHandle, $true).Dispose()
        }
        if ($nameStructure -ne [IntPtr]::Zero) {
            [Runtime.InteropServices.Marshal]::FreeHGlobal($nameStructure)
        }
        [Runtime.InteropServices.Marshal]::FreeHGlobal($nameBuffer)
    }
}

function Set-AmeR2cRBootstrapCaseSensitivityForGuardrail {
    param(
        [Parameter(Mandatory = $true)][psobject]$Bootstrap,
        [Parameter(Mandatory = $true)][bool]$Enabled
    )
    Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $Bootstrap
    $caseHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $Bootstrap.ToolHandle `
        -LeafName ([string]$Bootstrap.LeafName) `
        -Create $false `
        -RequestDelete $false `
        -RequestWriteAttributes
    try {
        $caseEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $caseHandle `
            -Label "case-sensitive source guardrail"
        if ($caseEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R case-sensitive source guardrail encountered a replacement identity"
        }
        $informationType = "AmeR2cRBootstrapDisposition" -as [type]
        $information = [Activator]::CreateInstance($informationType)
        $information.DeleteFile = if ($Enabled) { 1 } else { 0 }
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $caseHandle,
            23,
            [ref]$information,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($information)
        )) {
            $win32 = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "R2c-R could not set case-sensitive source guardrail semantics; Win32=$win32"
        }
        if ((Test-AmeR2cRBootstrapDirectoryCaseSensitive -Handle $caseHandle) -ne $Enabled) {
            throw "R2c-R case-sensitive source guardrail semantics did not take effect"
        }
    } finally { $caseHandle.Dispose() }
}

function Open-AmeR2cRBootstrapDirectoryChain {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $volumeRoot = [IO.Path]::GetPathRoot($fullPath)
    if ([string]::IsNullOrEmpty($volumeRoot) -or $volumeRoot -notmatch '^[A-Za-z]:\\$') {
        throw "R2c-R $Label must be on a local drive-letter volume"
    }
    $handles = [Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]]::new()
    try {
        $volumeHandle = $null
        try {
            $volumeHandle = [AmeR2cRBootstrapNative]::CreateFileW(
                $volumeRoot,
                [uint32](0x80 -bor 0x100000),
                [uint32](0x1 -bor 0x2),
                [IntPtr]::Zero,
                3,
                [uint32](0x200000 -bor 0x2000000),
                [IntPtr]::Zero
            )
            if ($volumeHandle.IsInvalid) {
                throw "R2c-R could not hold the $Label volume root"
            }
            Invoke-AmeR2cRCompilerFaultForGuardrail `
                -Point "volume-post-open-pre-transfer" `
                -Owner $volumeHandle
            $volumeEvidence = Get-AmeR2cRBootstrapHandleEvidence `
                -Handle $volumeHandle -Label "$Label volume root"
            $handles.Add($volumeHandle)
            $parent = $volumeHandle
            $volumeHandle = $null
        } finally {
            if ($null -ne $volumeHandle) { $volumeHandle.Dispose() }
        }
        $relative = $fullPath.Substring($volumeRoot.Length)
        foreach ($component in @(
            $relative.Split([char[]]@('\'), [StringSplitOptions]::RemoveEmptyEntries)
        )) {
            $child = Open-AmeR2cRBootstrapRelativeDirectory `
                -ParentHandle $parent `
                -LeafName $component `
                -Create $false `
                -RequestDelete $false `
                -ExistingComponent
            try {
                $childEvidence = Get-AmeR2cRBootstrapHandleEvidence `
                    -Handle $child -Label "$Label component"
                if ($childEvidence.VolumeSerialNumber -ne $volumeEvidence.VolumeSerialNumber) {
                    throw "R2c-R $Label crossed its trusted volume boundary"
                }
                $handles.Add($child)
                $parent = $child
                $child = $null
            } finally {
                if ($null -ne $child) { $child.Dispose() }
            }
        }
        $terminalEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $parent -Label "$Label terminal"
        return [pscustomobject]@{
            Handles = $handles
            Identity = [string]$terminalEvidence.Identity
            Path = [string]$terminalEvidence.Path
            Terminal = $parent
            VolumeSerialNumber = [uint32]$terminalEvidence.VolumeSerialNumber
        }
    } catch {
        for ($index = $handles.Count - 1; $index -ge 0; $index--) {
            $handles[$index].Dispose()
        }
        throw
    }
}

function Remove-AmeR2cRBootstrapExpectedDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.SafeHandles.SafeFileHandle]$ParentHandle,
        [Parameter(Mandatory = $true)][string]$LeafName,
        [Parameter(Mandatory = $true)][string]$ExpectedIdentity,
        [Parameter(Mandatory = $true)][bool]$ExpectReparse
    )
    $deleteHandle = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $ParentHandle `
        -LeafName $LeafName `
        -Create $false `
        -RequestDelete $true
    try {
        $deleteEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $deleteHandle `
            -Label "compiler guardrail cleanup" `
            -AllowReparse
        if ($deleteEvidence.Identity -cne $ExpectedIdentity -or
            [bool]$deleteEvidence.IsReparse -ne $ExpectReparse) {
            throw "R2c-R compiler guardrail cleanup encountered an unknown identity"
        }
        $dispositionType = "AmeR2cRBootstrapDisposition" -as [type]
        $disposition = [Activator]::CreateInstance($dispositionType)
        $disposition.DeleteFile = 1
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $deleteHandle,
            4,
            [ref]$disposition,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($disposition)
        )) {
            throw "R2c-R could not delete the exact compiler guardrail directory"
        }
    } finally { $deleteHandle.Dispose() }
}

function Assert-AmeR2cRCompilerBootstrapHeld {
    param([Parameter(Mandatory = $true)][psobject]$Bootstrap)
    if ($null -eq $Bootstrap.ToolAnchorHandles -or
        $Bootstrap.ToolAnchorHandles.Count -ne $Bootstrap.ToolAnchorIdentities.Count) {
        throw "R2c-R compiler bootstrap lost its held anchor chain"
    }
    for ($index = 0; $index -lt $Bootstrap.ToolAnchorHandles.Count; $index++) {
        $chainEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $Bootstrap.ToolAnchorHandles[$index] `
            -Label "compiler tool anchor chain"
        if ($chainEvidence.Identity -cne $Bootstrap.ToolAnchorIdentities[$index]) {
            throw "R2c-R compiler tool anchor chain identity changed while held"
        }
    }
    $toolEvidence = Get-AmeR2cRBootstrapHandleEvidence `
        -Handle $Bootstrap.ToolHandle -Label "compiler tool anchor"
    $rootEvidence = Get-AmeR2cRBootstrapHandleEvidence `
        -Handle $Bootstrap.RootHandle -Label "compiler bootstrap"
    if ($toolEvidence.Identity -cne $Bootstrap.ToolIdentity -or
        $rootEvidence.Identity -cne $Bootstrap.RootIdentity) {
        throw "R2c-R compiler bootstrap identity changed while held"
    }
    if ($rootEvidence.VolumeSerialNumber -ne $toolEvidence.VolumeSerialNumber) {
        throw "R2c-R compiler bootstrap escaped its held tool anchor"
    }
    $relativeRoot = Open-AmeR2cRBootstrapRelativeDirectory `
        -ParentHandle $Bootstrap.ToolHandle `
        -LeafName ([string]$Bootstrap.LeafName) `
        -Create $false `
        -RequestDelete $false
    try {
        $relativeEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $relativeRoot -Label "compiler bootstrap relative identity"
        if ($relativeEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R compiler bootstrap is no longer the held tool child"
        }
    } finally { $relativeRoot.Dispose() }
}

function New-AmeR2cRCompilerBootstrap {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [string]$LeafName = ".ame-r2c-r-bootstrap-$([Guid]::NewGuid().ToString('N'))"
    )
    Initialize-AmeR2cRBootstrapNativeTypes
    $toolPath = [IO.Path]::GetFullPath((Join-Path $RepositoryRoot "tool")).TrimEnd('\')
    $logicalChain = $null
    $physicalChain = $null
    $rootHandle = $null
    try {
        $logicalChain = Open-AmeR2cRBootstrapDirectoryChain `
            -Path $toolPath -Label "compiler logical tool anchor"
        $physicalChain = Open-AmeR2cRBootstrapDirectoryChain `
            -Path ([string]$logicalChain.Path) -Label "compiler physical tool anchor"
        if ($logicalChain.Identity -cne $physicalChain.Identity -or
            $logicalChain.VolumeSerialNumber -ne $physicalChain.VolumeSerialNumber) {
            throw "R2c-R compiler tool anchor changed during read-only physical binding"
        }
        $toolHandle = $physicalChain.Terminal
        $toolEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $toolHandle -Label "compiler tool anchor"
        $rootHandle = Open-AmeR2cRBootstrapRelativeDirectory `
            -ParentHandle $toolHandle -LeafName $LeafName -Create $true -RequestDelete $false
        $rootEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $rootHandle -Label "compiler bootstrap"
        if ($rootEvidence.VolumeSerialNumber -ne $toolEvidence.VolumeSerialNumber) {
            throw "R2c-R compiler bootstrap escaped its held physical tool anchor"
        }
        $anchorHandles = [Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]]::new()
        $anchorIdentities = [Collections.Generic.List[string]]::new()
        foreach ($handle in @($logicalChain.Handles) + @($physicalChain.Handles)) {
            $anchorHandles.Add($handle)
            $anchorIdentities.Add([string](
                Get-AmeR2cRBootstrapHandleEvidence `
                    -Handle $handle -Label "compiler retained anchor"
            ).Identity)
        }
        return [pscustomobject]@{
            LeafName = $LeafName
            Path = $rootEvidence.Path
            RootHandle = $rootHandle
            RootIdentity = $rootEvidence.Identity
            ToolHandle = $toolHandle
            ToolAnchorHandles = $anchorHandles
            ToolAnchorIdentities = $anchorIdentities
            ToolIdentity = $toolEvidence.Identity
            ToolPath = $toolEvidence.Path
        }
    } catch {
        if ($null -ne $rootHandle) { $rootHandle.Dispose() }
        foreach ($chain in @($physicalChain, $logicalChain)) {
            if ($null -ne $chain) {
                for ($index = $chain.Handles.Count - 1; $index -ge 0; $index--) {
                    $chain.Handles[$index].Dispose()
                }
            }
        }
        throw
    }
}

function Remove-AmeR2cRCompilerBootstrap {
    param([Parameter(Mandatory = $true)][psobject]$Bootstrap)
    $deleteHandle = $null
    try {
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $Bootstrap
        $entries = @(
            [IO.DirectoryInfo]::new([string]$Bootstrap.Path).EnumerateFileSystemInfos()
        )
        if ($entries.Count -ne 0) {
            throw (
                "R2c-R compiler bootstrap contains unknown residue and was retained " +
                "without traversal: $($Bootstrap.Path)"
            )
        }
        $Bootstrap.RootHandle.Dispose()
        $Bootstrap.RootHandle = $null
        $deleteHandle = Open-AmeR2cRBootstrapRelativeDirectory `
            -ParentHandle $Bootstrap.ToolHandle `
            -LeafName ([string]$Bootstrap.LeafName) `
            -Create $false `
            -RequestDelete $true
        $deleteEvidence = Get-AmeR2cRBootstrapHandleEvidence `
            -Handle $deleteHandle -Label "compiler bootstrap cleanup"
        if ($deleteEvidence.Identity -cne $Bootstrap.RootIdentity) {
            throw "R2c-R compiler bootstrap cleanup encountered a replacement identity"
        }
        $dispositionType = "AmeR2cRBootstrapDisposition" -as [type]
        $disposition = [Activator]::CreateInstance($dispositionType)
        $disposition.DeleteFile = 1
        if (-not [AmeR2cRBootstrapNative]::SetFileInformationByHandle(
            $deleteHandle,
            4,
            [ref]$disposition,
            [uint32][Runtime.InteropServices.Marshal]::SizeOf($disposition)
        )) {
            throw "R2c-R could not delete the exact compiler bootstrap by held identity"
        }
    } finally {
        if ($null -ne $deleteHandle) { $deleteHandle.Dispose() }
        if ($null -ne $Bootstrap.RootHandle) { $Bootstrap.RootHandle.Dispose() }
        if ($null -ne $Bootstrap.ToolAnchorHandles) {
            for ($index = $Bootstrap.ToolAnchorHandles.Count - 1; $index -ge 0; $index--) {
                $Bootstrap.ToolAnchorHandles[$index].Dispose()
            }
        }
        $Bootstrap.ToolHandle = $null
    }
}

function Initialize-AmeR2cRNativeTypes {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [switch]$ForceCompilerFailure
    )
    $nativeType = "AmeR2cRNative" -as [type]
    $jobType = "AmeR2cRProcessJob" -as [type]
    $rootType = "AmeR2cRDisposableRoot" -as [type]
    $snapshotType = "AmeR2cRAuditedScriptSnapshot" -as [type]
    if ($null -ne $nativeType -and $null -ne $jobType -and $null -ne $rootType -and
        $null -ne $snapshotType) { return }
    if ($null -ne $nativeType -or $null -ne $jobType -or $null -ne $rootType -or
        $null -ne $snapshotType) {
        throw "R2c-R native types were only partially initialized"
    }

    $bootstrap = $null
    $previousTemp = $null
    $previousTmp = $null
    $environmentCaptured = $false
    $environmentMutated = $false
    try {
        $bootstrap = New-AmeR2cRCompilerBootstrap -RepositoryRoot $RepositoryRoot
        Invoke-AmeR2cRCompilerFaultForGuardrail `
            -Point "bootstrap-post-create-pre-initialization" `
            -Owner $bootstrap
        $previousTemp = [Environment]::GetEnvironmentVariable("TEMP", "Process")
        $previousTmp = [Environment]::GetEnvironmentVariable("TMP", "Process")
        $environmentCaptured = $true
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $bootstrap
        $environmentMutated = $true
        [Environment]::SetEnvironmentVariable("TEMP", $bootstrap.Path, "Process")
        [Environment]::SetEnvironmentVariable("TMP", $bootstrap.Path, "Process")
        $nativeTypeDefinition = @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Win32.SafeHandles;

public static class AmeR2cRNative
{
    public static Version NativeVersion()
    {
        RTL_OSVERSIONINFOEX version = new RTL_OSVERSIONINFOEX();
        version.dwOSVersionInfoSize = (uint)Marshal.SizeOf(typeof(RTL_OSVERSIONINFOEX));
        int status = RtlGetVersion(ref version);
        if (status != 0) throw new InvalidOperationException("Could not query the native Windows version; NTSTATUS=0x" + status.ToString("X8"));
        return new Version((int)version.dwMajorVersion, (int)version.dwMinorVersion, (int)version.dwBuildNumber);
    }

    public static string ResolveExistingPath(string path)
    {
        using (SafeFileHandle handle = CreateFile(path, 0x00000080, 7, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero))
        {
            if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not resolve the R2c-R path");
            StringBuilder buffer = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(handle, buffer, (uint)buffer.Capacity, 0);
            if (length == 0 || length >= buffer.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the resolved R2c-R path");
            string resolved = buffer.ToString();
            if (resolved.StartsWith("\\\\?\\UNC\\", StringComparison.OrdinalIgnoreCase)) return "\\\\" + resolved.Substring(8);
            return resolved.StartsWith("\\\\?\\", StringComparison.OrdinalIgnoreCase) ? resolved.Substring(4) : resolved;
        }
    }

    public static uint ProductSku(uint major, uint minor)
    {
        uint product;
        if (!GetProductInfo(major, minor, 0, 0, out product)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query the Windows product SKU");
        return product;
    }

    public static void AssertDirectoryAnchor(string path)
    {
        AmeR2cRDisposableRoot.AssertDirectoryAnchor(path);
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(SafeFileHandle file, StringBuilder filePath, uint filePathLength, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetProductInfo(uint majorVersion, uint minorVersion, uint servicePackMajor, uint servicePackMinor, out uint productType);
    [DllImport("ntdll.dll", CharSet = CharSet.Unicode)]
    private static extern int RtlGetVersion(ref RTL_OSVERSIONINFOEX versionInformation);
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct RTL_OSVERSIONINFOEX
    {
        public uint dwOSVersionInfoSize;
        public uint dwMajorVersion;
        public uint dwMinorVersion;
        public uint dwBuildNumber;
        public uint dwPlatformId;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szCSDVersion;
        public ushort wServicePackMajor;
        public ushort wServicePackMinor;
        public ushort wSuiteMask;
        public byte wProductType;
        public byte wReserved;
    }
}

public sealed class AmeR2cRDisposableRoot : IDisposable
{
    private const uint FILE_LIST_DIRECTORY = 0x00000001;
    private const uint FILE_TRAVERSE = 0x00000020;
    private const uint FILE_READ_ATTRIBUTES = 0x00000080;
    private const uint SYNCHRONIZE = 0x00100000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
    private const uint FILE_ATTRIBUTE_DIRECTORY = 0x00000010;
    private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
    private const uint DRIVE_FIXED = 3;
    private static readonly Guid LocalApplicationData = new Guid("F1B32785-6FBA-4FCF-9D55-7B8E7F157091");
    private static string guardrailFaultPoint;
    private static SafeFileHandle guardrailFaultHandle;

    private SafeFileHandle anchorHandle;
    private SafeFileHandle rootHandle;
    private readonly List<SafeFileHandle> trustAnchorHandles = new List<SafeFileHandle>();
    private readonly List<DirectoryIdentity> trustAnchorIdentities = new List<DirectoryIdentity>();
    private DirectoryIdentity anchorIdentity;
    private DirectoryIdentity rootIdentity;

    public string Path { get; private set; }
    public string AnchorPath { get; private set; }
    public string KnownFolderPath { get; private set; }
    public string RootName { get; private set; }
    public string VolumeRoot { get; private set; }
    public string VolumeFormat { get; private set; }
    public string Nonce { get; private set; }
    public string RootIdentityToken { get { return rootIdentity.ToString(); } }
    public bool IsRootHandleHeld { get { return rootHandle != null && !rootHandle.IsClosed; } }

    private AmeR2cRDisposableRoot() { }

    public static AmeR2cRDisposableRoot Create(string nonce)
    {
        return CreateAtAnchor(ResolveKnownFolderPath(LocalApplicationData), nonce);
    }

    public static AmeR2cRDisposableRoot CreateForGuardrail(string anchorPath, string nonce)
    {
        if (String.IsNullOrWhiteSpace(anchorPath) || !System.IO.Path.IsPathRooted(anchorPath)) throw new ArgumentException("The R2c-R guardrail anchor must be an absolute path", "anchorPath");
        return CreateAtAnchor(System.IO.Path.GetFullPath(anchorPath).TrimEnd('\\'), nonce);
    }

    public static AmeR2cRDisposableRoot CreateForKnownFolderGuardrail(string anchorPath, string nonce)
    {
        if (String.IsNullOrWhiteSpace(anchorPath) || !System.IO.Path.IsPathRooted(anchorPath)) throw new ArgumentException("The R2c-R KnownFolder guardrail anchor must be an absolute path", "anchorPath");
        return CreateAtAnchor(System.IO.Path.GetFullPath(anchorPath).TrimEnd('\\'), nonce);
    }

    private static AmeR2cRDisposableRoot CreateAtAnchor(string requestedAnchor, string nonce)
    {
        if (nonce == null || nonce.Length != 32) throw new ArgumentException("The R2c-R nonce must contain 32 lowercase hexadecimal characters", "nonce");
        foreach (char value in nonce)
        {
            if (!((value >= '0' && value <= '9') || (value >= 'a' && value <= 'f'))) throw new ArgumentException("The R2c-R nonce must contain 32 lowercase hexadecimal characters", "nonce");
        }

        AmeR2cRDisposableRoot fixture = new AmeR2cRDisposableRoot();
        Exception cleanupFailure = null;
        try
        {
            fixture.KnownFolderPath = System.IO.Path.GetFullPath(requestedAnchor).TrimEnd('\\');
            SafeFileHandle logicalAnchor = fixture.BindDirectoryChain(fixture.KnownFolderPath, "logical KnownFolder anchor");
            string physicalPath = FinalPath(logicalAnchor);
            SafeFileHandle physicalAnchor = fixture.BindDirectoryChain(physicalPath, "physical KnownFolder anchor");
            if (!Identity(logicalAnchor).Equals(Identity(physicalAnchor))) throw new InvalidOperationException("R2c-R KnownFolder identity changed during read-only physical binding");
            fixture.anchorHandle = physicalAnchor;
            fixture.AnchorPath = FinalPath(fixture.anchorHandle);
            fixture.anchorIdentity = Identity(fixture.anchorHandle);
            fixture.VolumeRoot = System.IO.Path.GetPathRoot(fixture.AnchorPath);
            fixture.VolumeFormat = AssertFixedNtfs(fixture.anchorHandle, fixture.VolumeRoot);

            fixture.Nonce = nonce;
            fixture.RootName = "Cedarflake-Ame-R2c-R-" + nonce + "-" + Guid.NewGuid().ToString("N");
            fixture.rootHandle = OpenRelativeDirectory(fixture.anchorHandle, fixture.RootName, true, false);
            AssertNonReparseDirectory(fixture.rootHandle, "R2c-R disposable root");
            fixture.Path = FinalPath(fixture.rootHandle);
            fixture.rootIdentity = Identity(fixture.rootHandle);
            fixture.ValidateHeldIdentity();
            return fixture;
        }
        catch (Exception failure)
        {
            if (fixture.rootHandle != null)
            {
                try { fixture.DeleteFailedEmptyRoot(); }
                catch (Exception cleanup) { cleanupFailure = cleanup; }
            }
            fixture.DisposeTrustAnchors();
            if (cleanupFailure != null) throw new AggregateException("R2c-R root creation and local cleanup both failed", failure, cleanupFailure);
            throw;
        }
    }

    public static void AssertOwnedLeafName(string name)
    {
        ValidateOwnedLeafName(name);
    }

    public static void AssertDirectoryAnchor(string path)
    {
        AmeR2cRDisposableRoot probe = new AmeR2cRDisposableRoot();
        try
        {
            SafeFileHandle logical = probe.BindDirectoryChain(System.IO.Path.GetFullPath(path), "guardrail logical anchor");
            SafeFileHandle physical = probe.BindDirectoryChain(FinalPath(logical), "guardrail physical anchor");
            if (!Identity(logical).Equals(Identity(physical))) throw new InvalidOperationException("R2c-R guardrail anchor changed during physical binding");
        }
        finally { probe.DisposeTrustAnchors(); }
    }

    public static bool GuardrailFaultHandleIsClosed
    {
        get { return guardrailFaultHandle == null || guardrailFaultHandle.IsClosed; }
    }

    public static void ProbeBindDirectoryChainPostOpenFaultForGuardrail(string path)
    {
        AmeR2cRDisposableRoot probe = new AmeR2cRDisposableRoot();
        BeginGuardrailFault("bind-volume-post-open");
        try
        {
            probe.BindDirectoryChain(System.IO.Path.GetFullPath(path), "guardrail fault probe");
            throw new InvalidOperationException("R2c-R BindDirectoryChain guardrail fault was not injected");
        }
        finally
        {
            EndGuardrailFault();
            probe.DisposeTrustAnchors();
        }
    }

    public void ProbeDeleteFailedEmptyRootPostOpenFaultForGuardrail()
    {
        BeginGuardrailFault("delete-failed-root-post-open");
        try
        {
            DeleteFailedEmptyRoot();
            throw new InvalidOperationException("R2c-R DeleteFailedEmptyRoot guardrail fault was not injected");
        }
        finally { EndGuardrailFault(); }
    }

    public static void ReleaseGuardrailFaultHandleForCleanup()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null) captured.Dispose();
    }

    public void ReleaseDeleteFailedGuardrailFaultHandleForCleanup()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null)
        {
            if (Object.ReferenceEquals(rootHandle, captured)) rootHandle = null;
            captured.Dispose();
        }
    }

    public void ValidateHeldIdentity()
    {
        if (anchorHandle == null || anchorHandle.IsClosed || rootHandle == null || rootHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        AssertNonReparseDirectory(anchorHandle, "held LocalApplicationData trust anchor");
        if (trustAnchorHandles.Count == 0 || trustAnchorHandles.Count != trustAnchorIdentities.Count) throw new InvalidOperationException("R2c-R held KnownFolder chain is incomplete");
        for (int index = 0; index < trustAnchorHandles.Count; index++)
        {
            SafeFileHandle handle = trustAnchorHandles[index];
            AssertNonReparseDirectory(handle, "held LocalApplicationData physical anchor chain");
            DirectoryIdentity identity = Identity(handle);
            if (!identity.Equals(trustAnchorIdentities[index]) || identity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R held KnownFolder chain identity changed");
        }
        AssertNonReparseDirectory(rootHandle, "held R2c-R disposable root");
        if (!Identity(anchorHandle).Equals(anchorIdentity) || !Identity(rootHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R held directory file identity changed");
        using (SafeFileHandle currentRoot = OpenRelativeDirectory(anchorHandle, RootName, false, false))
        {
            AssertNonReparseDirectory(currentRoot, "reopened R2c-R root child");
            if (!Identity(currentRoot).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R root is no longer the identity-bound child of its held KnownFolder anchor");
        }
        if (Identity(rootHandle).VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R root crossed its trusted volume boundary");
    }

    public void ReleaseRootHandleForCleanup()
    {
        ValidateHeldIdentity();
        rootHandle.Dispose();
        rootHandle = null;
    }

    public void DeleteEmptyRootForCleanup()
    {
        ReleaseRootHandleForCleanup();
        DeleteReleasedRootForCleanup();
    }

    public void DeleteReleasedRootForCleanup()
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R cleanup root blocker was not released");
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, RootName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "R2c-R cleanup root");
            if (!Identity(deleteHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R cleanup root identity changed after releasing the replacement blocker");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public void DeleteRelocatedExpectedRootForGuardrail(string leafName)
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R guardrail root blocker was not released");
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, leafName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "relocated R2c-R guardrail root");
            if (!Identity(deleteHandle).Equals(rootIdentity)) throw new InvalidOperationException("R2c-R guardrail relocated root did not retain the fixture identity");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public string CaptureGuardrailReplacementIdentity(string leafName, bool expectReparse)
    {
        AssertHeldAnchorForReleasedCleanup();
        using (SafeFileHandle handle = OpenRelativeDirectory(anchorHandle, leafName, false, false))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail replacement");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail replacement");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail replacement crossed the held fixture volume");
            return identity.ToString();
        }
    }

    public void DeleteGuardrailReplacementForCleanup(string leafName, string expectedIdentityToken, bool expectReparse)
    {
        AssertHeldAnchorForReleasedCleanup();
        if (rootHandle != null) throw new InvalidOperationException("R2c-R guardrail root blocker was not released");
        DirectoryIdentity expectedIdentity = DirectoryIdentity.Parse(expectedIdentityToken);
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, leafName, false, true))
        {
            if (expectReparse) AssertReparseDirectory(deleteHandle, "R2c-R guardrail replacement");
            else AssertNonReparseDirectory(deleteHandle, "R2c-R guardrail replacement");
            DirectoryIdentity actualIdentity = Identity(deleteHandle);
            if (!actualIdentity.Equals(expectedIdentity) || actualIdentity.VolumeSerialNumber != anchorIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail replacement identity changed before teardown");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    public string CreateGuardrailChildDirectory(string leafName)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, true, false))
        {
            AssertNonReparseDirectory(handle, "R2c-R guardrail child");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child crossed its fixture volume");
            return identity.ToString();
        }
    }

    public string CaptureGuardrailChildDirectoryIdentity(string leafName, bool expectReparse)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, false, false))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail child");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail child");
            DirectoryIdentity identity = Identity(handle);
            if (identity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child crossed its fixture volume");
            return identity.ToString();
        }
    }

    public void DeleteGuardrailChildDirectoryForCleanup(string leafName, string expectedIdentityToken, bool expectReparse)
    {
        ValidateGuardrailLeafName(leafName);
        ValidateHeldIdentity();
        DirectoryIdentity expectedIdentity = DirectoryIdentity.Parse(expectedIdentityToken);
        using (SafeFileHandle handle = OpenRelativeDirectoryComponent(rootHandle, leafName, false, true))
        {
            if (expectReparse) AssertReparseDirectory(handle, "R2c-R guardrail child cleanup");
            else AssertNonReparseDirectory(handle, "R2c-R guardrail child cleanup");
            DirectoryIdentity actualIdentity = Identity(handle);
            if (!actualIdentity.Equals(expectedIdentity) || actualIdentity.VolumeSerialNumber != rootIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R guardrail child identity changed before teardown");
            DeleteEmptyDirectoryHandle(handle);
        }
    }

    public void Dispose()
    {
        if (rootHandle != null) { rootHandle.Dispose(); rootHandle = null; }
        DisposeTrustAnchors();
    }

    private SafeFileHandle BindDirectoryChain(string path, string label)
    {
        string fullPath = System.IO.Path.GetFullPath(path).TrimEnd('\\');
        string volumeRoot = System.IO.Path.GetPathRoot(fullPath);
        if (String.IsNullOrEmpty(volumeRoot) || !Regex.IsMatch(volumeRoot, @"\A[A-Za-z]:\\\z", RegexOptions.CultureInvariant)) throw new InvalidOperationException("R2c-R " + label + " must be on a local drive-letter volume");
        SafeFileHandle volumeHandle = OpenDirectoryWithoutDeleteSharing(volumeRoot, "Could not hold the R2c-R " + label + " volume root");
        SafeFileHandle parent;
        DirectoryIdentity volumeIdentity;
        try
        {
            ThrowIfGuardrailFault("bind-volume-post-open", volumeHandle);
            AssertNonReparseDirectory(volumeHandle, "R2c-R " + label + " volume root");
            volumeIdentity = Identity(volumeHandle);
            HoldTrustAnchor(volumeHandle);
            parent = volumeHandle;
            volumeHandle = null;
        }
        finally { if (volumeHandle != null) volumeHandle.Dispose(); }
        string relative = fullPath.Substring(volumeRoot.Length);
        foreach (string component in relative.Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries))
        {
            SafeFileHandle child = OpenRelativeDirectoryComponent(parent, component, false, false);
            try
            {
                AssertNonReparseDirectory(child, "R2c-R " + label + " component");
                DirectoryIdentity childIdentity = Identity(child);
                if (childIdentity.VolumeSerialNumber != volumeIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R " + label + " crossed its trusted volume boundary");
                HoldTrustAnchor(child);
                parent = child;
                child = null;
            }
            finally { if (child != null) child.Dispose(); }
        }
        return parent;
    }

    private void HoldTrustAnchor(SafeFileHandle handle)
    {
        DirectoryIdentity identity = Identity(handle);
        trustAnchorHandles.Add(handle);
        try { trustAnchorIdentities.Add(identity); }
        catch
        {
            trustAnchorHandles.RemoveAt(trustAnchorHandles.Count - 1);
            throw;
        }
    }

    private void DisposeTrustAnchors()
    {
        for (int index = trustAnchorHandles.Count - 1; index >= 0; index--) trustAnchorHandles[index].Dispose();
        trustAnchorHandles.Clear();
        trustAnchorIdentities.Clear();
        anchorHandle = null;
    }

    private void AssertHeldAnchorForReleasedCleanup()
    {
        if (anchorHandle == null || anchorHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        if (trustAnchorHandles.Count == 0 || trustAnchorHandles.Count != trustAnchorIdentities.Count) throw new InvalidOperationException("R2c-R cleanup anchor chain is incomplete");
        for (int index = 0; index < trustAnchorHandles.Count; index++)
        {
            SafeFileHandle handle = trustAnchorHandles[index];
            AssertNonReparseDirectory(handle, "held LocalApplicationData physical cleanup anchor");
            if (!Identity(handle).Equals(trustAnchorIdentities[index])) throw new InvalidOperationException("R2c-R cleanup anchor chain identity changed");
        }
        AssertNonReparseDirectory(anchorHandle, "held LocalApplicationData cleanup anchor");
        if (!Identity(anchorHandle).Equals(anchorIdentity)) throw new InvalidOperationException("R2c-R cleanup anchor identity changed");
    }

    private static string ResolveKnownFolderPath(Guid identifier)
    {
        IntPtr rawPath;
        int result = SHGetKnownFolderPath(ref identifier, 0, IntPtr.Zero, out rawPath);
        if (result != 0) Marshal.ThrowExceptionForHR(result);
        try
        {
            string path = Marshal.PtrToStringUni(rawPath);
            if (String.IsNullOrWhiteSpace(path) || !System.IO.Path.IsPathRooted(path)) throw new InvalidOperationException("Could not resolve the LocalApplicationData KnownFolder path");
            return path;
        }
        finally { Marshal.FreeCoTaskMem(rawPath); }
    }

    private static SafeFileHandle OpenDirectoryWithoutDeleteSharing(string path, string message)
    {
        SafeFileHandle handle = CreateFileW(path, FILE_READ_ATTRIBUTES | SYNCHRONIZE, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero);
        if (handle.IsInvalid)
        {
            int error = Marshal.GetLastWin32Error();
            handle.Dispose();
            throw new Win32Exception(error, message);
        }
        return handle;
    }

    private void DeleteFailedEmptyRoot()
    {
        SafeFileHandle failedRootHandle = rootHandle;
        rootHandle = null;
        if (failedRootHandle == null) throw new ObjectDisposedException("AmeR2cRDisposableRoot");
        DirectoryIdentity expectedIdentity;
        try
        {
            ThrowIfGuardrailFault("delete-failed-root-post-open", failedRootHandle);
            expectedIdentity = Identity(failedRootHandle);
        }
        finally { failedRootHandle.Dispose(); }
        using (SafeFileHandle deleteHandle = OpenRelativeDirectory(anchorHandle, RootName, false, true))
        {
            AssertNonReparseDirectory(deleteHandle, "failed R2c-R root cleanup");
            if (!Identity(deleteHandle).Equals(expectedIdentity)) throw new InvalidOperationException("Failed R2c-R root cleanup encountered a replacement identity");
            DeleteEmptyDirectoryHandle(deleteHandle);
        }
    }

    private static void BeginGuardrailFault(string point)
    {
        if (guardrailFaultPoint != null || guardrailFaultHandle != null) throw new InvalidOperationException("R2c-R guardrail fault state was not clean");
        guardrailFaultPoint = point;
    }

    private static void EndGuardrailFault()
    {
        guardrailFaultPoint = null;
    }

    private static void ThrowIfGuardrailFault(string point, SafeFileHandle handle)
    {
        if (!String.Equals(guardrailFaultPoint, point, StringComparison.Ordinal)) return;
        guardrailFaultHandle = handle;
        throw new InvalidOperationException("R2c-R injected post-open handle fault: " + point);
    }

    private static SafeFileHandle OpenRelativeDirectory(SafeFileHandle parent, string name, bool create, bool requestDelete)
    {
        ValidateOwnedLeafName(name);
        return OpenRelativeDirectoryComponentCore(parent, name, create, requestDelete, true);
    }

    private static SafeFileHandle OpenRelativeDirectoryComponent(SafeFileHandle parent, string name, bool create, bool requestDelete)
    {
        return OpenRelativeDirectoryComponentCore(parent, name, create, requestDelete, false);
    }

    private static SafeFileHandle OpenRelativeDirectoryComponentCore(SafeFileHandle parent, string name, bool create, bool requestDelete, bool holdDeleteBlocker)
    {
        ValidateDirectoryComponent(name);
        IntPtr nameBuffer = Marshal.StringToHGlobalUni(name);
        IntPtr nameStructure = IntPtr.Zero;
        IntPtr rawHandle = IntPtr.Zero;
        try
        {
            UNICODE_STRING unicodeName = new UNICODE_STRING();
            unicodeName.Length = checked((ushort)(name.Length * 2));
            unicodeName.MaximumLength = checked((ushort)((name.Length + 1) * 2));
            unicodeName.Buffer = nameBuffer;
            nameStructure = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
            Marshal.StructureToPtr(unicodeName, nameStructure, false);
            OBJECT_ATTRIBUTES attributes = new OBJECT_ATTRIBUTES();
            attributes.Length = (uint)Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES));
            attributes.RootDirectory = parent.DangerousGetHandle();
            attributes.ObjectName = nameStructure;
            attributes.Attributes = 0;
            if (!DirectoryIsCaseSensitive(parent)) attributes.Attributes |= 0x00000040;
            IO_STATUS_BLOCK statusBlock;
            uint desiredAccess = FILE_READ_ATTRIBUTES | SYNCHRONIZE;
            if (holdDeleteBlocker) desiredAccess |= FILE_LIST_DIRECTORY;
            if (requestDelete) desiredAccess |= 0x00010000;
            int status = NtCreateFile(out rawHandle, desiredAccess, ref attributes, out statusBlock, IntPtr.Zero, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ | FILE_SHARE_WRITE, create ? 2u : 1u, 0x00000001 | 0x00000020 | 0x00004000 | 0x00200000, IntPtr.Zero, 0);
            if (status < 0)
            {
                uint error = RtlNtStatusToDosError(status);
                string operation = create ? "create" : "reopen";
                throw new Win32Exception(unchecked((int)error), "Could not " + operation + " the R2c-R root relative to its held KnownFolder anchor; NTSTATUS=0x" + status.ToString("X8") + " Win32=" + error.ToString());
            }
            SafeFileHandle handle = new SafeFileHandle(rawHandle, true);
            rawHandle = IntPtr.Zero;
            return handle;
        }
        finally
        {
            if (rawHandle != IntPtr.Zero && rawHandle != new IntPtr(-1)) CloseHandle(rawHandle);
            if (nameStructure != IntPtr.Zero) Marshal.FreeHGlobal(nameStructure);
            Marshal.FreeHGlobal(nameBuffer);
        }
    }

    private static bool DirectoryIsCaseSensitive(SafeFileHandle directory)
    {
        FILE_CASE_SENSITIVE_INFO information = new FILE_CASE_SENSITIVE_INFO();
        if (!GetFileInformationByHandleEx(directory, 23, out information, (uint)Marshal.SizeOf(typeof(FILE_CASE_SENSITIVE_INFO)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query R2c-R parent case semantics");
        return (information.Flags & 1) != 0;
    }

    private static void ValidateDirectoryComponent(string name)
    {
        if (String.IsNullOrEmpty(name) || name == "." || name == ".." || name.Length > 255 ||
            name.IndexOfAny(new char[] { '\0', '\\', '/', ':' }) >= 0 ||
            name[name.Length - 1] == '.' || name[name.Length - 1] == ' ') throw new ArgumentException("R2c-R directory component is not one normalized NT name", "name");
    }

    private static void ValidateGuardrailLeafName(string name)
    {
        ValidateDirectoryComponent(name);
        if (!Regex.IsMatch(name, @"\A[a-z][a-z0-9-]{0,63}-[0-9a-f]{32}\z", RegexOptions.CultureInvariant)) throw new ArgumentException("R2c-R guardrail leaf is outside the identity-bound owned-name contract", "name");
    }

    private static void ValidateOwnedLeafName(string name)
    {
        ValidateDirectoryComponent(name);
        if (String.IsNullOrEmpty(name) || name.Length > 160 ||
            !name.StartsWith("Cedarflake-Ame-R2c-R-", StringComparison.Ordinal)) throw new ArgumentException("R2c-R relative directory name is outside the owned-name contract", "name");
        foreach (char value in name)
        {
            if (!((value >= 'A' && value <= 'Z') ||
                  (value >= 'a' && value <= 'z') ||
                  (value >= '0' && value <= '9') || value == '-')) throw new ArgumentException("R2c-R relative directory name contains path, ADS, control, or non-ASCII syntax", "name");
        }
        if (name == "." || name == ".." || name[name.Length - 1] == '.' ||
            name[name.Length - 1] == ' ') throw new ArgumentException("R2c-R relative directory name has a forbidden terminal component", "name");
        if (!Regex.IsMatch(
            name,
            @"\ACedarflake-Ame-R2c-R-[0-9a-f]{32}-[0-9a-f]{32}(?:-moved-[0-9a-f]{32})?\z",
            RegexOptions.CultureInvariant)) throw new ArgumentException("R2c-R relative directory name is not an exact generated owned leaf", "name");
    }

    private static void DeleteEmptyDirectoryHandle(SafeFileHandle handle)
    {
        FILE_DISPOSITION_INFO disposition = new FILE_DISPOSITION_INFO();
        disposition.DeleteFile = true;
        if (!SetFileInformationByHandle(handle, 4, ref disposition, (uint)Marshal.SizeOf(typeof(FILE_DISPOSITION_INFO)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not delete the exact failed or completed R2c-R root by held handle");
    }

    private static void AssertNonReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("The " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("The " + label + " is a reparse point");
    }

    private static void AssertReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("The " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) == 0) throw new InvalidOperationException("The " + label + " is not a reparse point");
    }

    private static DirectoryIdentity Identity(SafeFileHandle handle)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the R2c-R directory identity");
        return new DirectoryIdentity(information.VolumeSerialNumber, information.FileIndexHigh, information.FileIndexLow);
    }

    private static string FinalPath(SafeFileHandle handle)
    {
        StringBuilder buffer = new StringBuilder(32768);
        uint length = GetFinalPathNameByHandleW(handle, buffer, (uint)buffer.Capacity, 0);
        if (length == 0 || length >= buffer.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not resolve the R2c-R directory handle");
        string resolved = buffer.ToString();
        if (resolved.StartsWith("\\\\?\\UNC\\", StringComparison.OrdinalIgnoreCase)) return "\\\\" + resolved.Substring(8);
        return resolved.StartsWith("\\\\?\\", StringComparison.OrdinalIgnoreCase) ? resolved.Substring(4) : resolved;
    }

    private static string AssertFixedNtfs(SafeFileHandle handle, string volumeRoot)
    {
        if (String.IsNullOrEmpty(volumeRoot) || GetDriveTypeW(volumeRoot) != DRIVE_FIXED) throw new InvalidOperationException("R2c-R disposable storage requires a fixed volume");
        StringBuilder volumeName = new StringBuilder(261);
        StringBuilder fileSystemName = new StringBuilder(261);
        uint serial;
        uint maximumComponentLength;
        uint flags;
        if (!GetVolumeInformationByHandleW(handle, volumeName, (uint)volumeName.Capacity, out serial, out maximumComponentLength, out flags, fileSystemName, (uint)fileSystemName.Capacity)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R KnownFolder volume");
        string format = fileSystemName.ToString();
        if (!String.Equals(format, "NTFS", StringComparison.Ordinal)) throw new InvalidOperationException("R2c-R disposable storage requires NTFS");
        return format;
    }

    private struct DirectoryIdentity : IEquatable<DirectoryIdentity>
    {
        internal readonly uint VolumeSerialNumber;
        private readonly uint FileIndexHigh;
        private readonly uint FileIndexLow;
        internal DirectoryIdentity(uint volumeSerialNumber, uint fileIndexHigh, uint fileIndexLow) { VolumeSerialNumber = volumeSerialNumber; FileIndexHigh = fileIndexHigh; FileIndexLow = fileIndexLow; }
        public bool Equals(DirectoryIdentity other) { return VolumeSerialNumber == other.VolumeSerialNumber && FileIndexHigh == other.FileIndexHigh && FileIndexLow == other.FileIndexLow; }
        public override bool Equals(object value) { return value is DirectoryIdentity && Equals((DirectoryIdentity)value); }
        public override int GetHashCode() { return VolumeSerialNumber.GetHashCode() ^ FileIndexHigh.GetHashCode() ^ FileIndexLow.GetHashCode(); }
        public override string ToString() { return VolumeSerialNumber.ToString("X8") + ":" + FileIndexHigh.ToString("X8") + ":" + FileIndexLow.ToString("X8"); }
        internal static DirectoryIdentity Parse(string value)
        {
            string[] fields = (value ?? String.Empty).Split(':');
            if (fields.Length != 3) throw new ArgumentException("R2c-R expected identity token is invalid", "value");
            uint volume;
            uint high;
            uint low;
            if (!UInt32.TryParse(fields[0], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out volume) ||
                !UInt32.TryParse(fields[1], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out high) ||
                !UInt32.TryParse(fields[2], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out low)) throw new ArgumentException("R2c-R expected identity token is invalid", "value");
            return new DirectoryIdentity(volume, high, low);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FILETIME { public uint LowDateTime; public uint HighDateTime; }
    [StructLayout(LayoutKind.Sequential)]
    private struct BY_HANDLE_FILE_INFORMATION
    {
        public uint FileAttributes;
        public FILETIME CreationTime;
        public FILETIME LastAccessTime;
        public FILETIME LastWriteTime;
        public uint VolumeSerialNumber;
        public uint FileSizeHigh;
        public uint FileSizeLow;
        public uint NumberOfLinks;
        public uint FileIndexHigh;
        public uint FileIndexLow;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct UNICODE_STRING { public ushort Length; public ushort MaximumLength; public IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)]
    private struct OBJECT_ATTRIBUTES { public uint Length; public IntPtr RootDirectory; public IntPtr ObjectName; public uint Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }
    [StructLayout(LayoutKind.Sequential)]
    private struct IO_STATUS_BLOCK { public IntPtr Status; public UIntPtr Information; }
    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_DISPOSITION_INFO { [MarshalAs(UnmanagedType.Bool)] public bool DeleteFile; }
    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_CASE_SENSITIVE_INFO { public uint Flags; }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] private static extern int SHGetKnownFolderPath(ref Guid knownFolder, uint flags, IntPtr token, out IntPtr path);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern SafeFileHandle CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandle(SafeFileHandle file, out BY_HANDLE_FILE_INFORMATION information);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandleEx(SafeFileHandle file, uint informationClass, out FILE_CASE_SENSITIVE_INFO information, uint bufferSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetFileInformationByHandle(SafeFileHandle file, uint informationClass, ref FILE_DISPOSITION_INFO information, uint bufferSize);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern uint GetFinalPathNameByHandleW(SafeFileHandle file, StringBuilder filePath, uint filePathLength, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern uint GetDriveTypeW(string rootPathName);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool GetVolumeInformationByHandleW(SafeFileHandle file, StringBuilder volumeNameBuffer, uint volumeNameSize, out uint volumeSerialNumber, out uint maximumComponentLength, out uint fileSystemFlags, StringBuilder fileSystemNameBuffer, uint fileSystemNameSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")] private static extern int NtCreateFile(out IntPtr fileHandle, uint desiredAccess, ref OBJECT_ATTRIBUTES objectAttributes, out IO_STATUS_BLOCK ioStatusBlock, IntPtr allocationSize, uint fileAttributes, uint shareAccess, uint createDisposition, uint createOptions, IntPtr eaBuffer, uint eaLength);
    [DllImport("ntdll.dll")] private static extern uint RtlNtStatusToDosError(int status);
}

public sealed class AmeR2cRAuditedScriptSnapshot : IDisposable
{
    private const uint FILE_READ_DATA = 0x00000001;
    private const uint FILE_TRAVERSE = 0x00000020;
    private const uint FILE_READ_ATTRIBUTES = 0x00000080;
    private const uint SYNCHRONIZE = 0x00100000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
    private const uint FILE_ATTRIBUTE_DIRECTORY = 0x00000010;
    private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
    private const uint FILE_DIRECTORY_FILE = 0x00000001;
    private const uint FILE_SYNCHRONOUS_IO_NONALERT = 0x00000020;
    private const uint FILE_OPEN_FOR_BACKUP_INTENT = 0x00004000;
    private const uint FILE_OPEN_REPARSE_POINT = 0x00200000;

    private readonly List<SafeFileHandle> directoryHandles = new List<SafeFileHandle>();
    private readonly List<ScriptIdentity> directoryIdentities = new List<ScriptIdentity>();
    private SafeFileHandle terminalHandle;
    private FileStream input;
    private ScriptIdentity terminalIdentity;
    private string terminalLeaf;
    private bool disposedForGuardrail;
    private static string guardrailFaultPoint;
    private static SafeFileHandle guardrailFaultHandle;
    private static int guardrailOpenCount;
    private static int guardrailDisposeCount;

    public string Path { get; private set; }
    public long Length { get; private set; }
    public string IdentityToken { get { return terminalIdentity.ToString(); } }

    private AmeR2cRAuditedScriptSnapshot() { }

    public static AmeR2cRAuditedScriptSnapshot Open(string toolRoot, string path)
    {
        string fullToolRoot = System.IO.Path.GetFullPath(toolRoot).TrimEnd('\\');
        string fullPath = System.IO.Path.GetFullPath(path);
        if (!String.Equals(System.IO.Path.GetExtension(fullPath), ".ps1", StringComparison.Ordinal)) throw new InvalidOperationException("R2c-R audited script must have the exact .ps1 extension");
        System.Threading.Interlocked.Increment(ref guardrailOpenCount);

        AmeR2cRAuditedScriptSnapshot snapshot = new AmeR2cRAuditedScriptSnapshot();
        try
        {
            SafeFileHandle toolHandle = snapshot.BindDirectoryChain(fullToolRoot);
            bool caseSensitive = DirectoryIsCaseSensitive(toolHandle);
            StringComparison comparison = caseSensitive ? StringComparison.Ordinal : StringComparison.OrdinalIgnoreCase;
            string toolPrefix = fullToolRoot + "\\";
            if (!fullPath.StartsWith(toolPrefix, comparison)) throw new InvalidOperationException("R2c-R audited script must remain below the held repository tool root");
            string[] components = fullPath.Substring(toolPrefix.Length).Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries);
            if (components.Length == 0) throw new InvalidOperationException("R2c-R audited script path has no terminal component");
            for (int index = 0; index < components.Length - 1; index++)
            {
                SafeFileHandle child = OpenRelative(toolHandle, components[index], true);
                try
                {
                    snapshot.HoldDirectory(child, "audited-script source parent");
                    ScriptIdentity childIdentity = snapshot.directoryIdentities[snapshot.directoryIdentities.Count - 1];
                    if (childIdentity.VolumeSerialNumber != snapshot.directoryIdentities[0].VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script source parent crossed its held volume");
                    ThrowIfGuardrailFault("parent-post-open", child);
                    toolHandle = child;
                    child = null;
                }
                finally { if (child != null) child.Dispose(); }
            }
            snapshot.terminalLeaf = components[components.Length - 1];
            ValidateComponent(snapshot.terminalLeaf);
            snapshot.terminalHandle = OpenRelative(toolHandle, snapshot.terminalLeaf, false);
            ThrowIfGuardrailFault("terminal-post-open", snapshot.terminalHandle);
            BY_HANDLE_FILE_INFORMATION information = AssertRegularNonReparseFile(snapshot.terminalHandle, "audited script terminal");
            snapshot.terminalIdentity = Identity(information);
            ScriptIdentity toolIdentity = snapshot.directoryIdentities[snapshot.directoryIdentities.Count - 1];
            if (snapshot.terminalIdentity.VolumeSerialNumber != toolIdentity.VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script crossed its held tool-root volume");
            snapshot.Length = checked(((long)information.FileSizeHigh << 32) | information.FileSizeLow);
            snapshot.Path = fullPath;
            snapshot.input = new FileStream(snapshot.terminalHandle, FileAccess.Read, 4096, false);
            snapshot.Revalidate();
            return snapshot;
        }
        catch
        {
            snapshot.Dispose();
            throw;
        }
    }

    public string ReadUtf8Text()
    {
        if (input == null || terminalHandle == null || terminalHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRAuditedScriptSnapshot");
        input.Position = 0;
        using (StreamReader reader = new StreamReader(input, new UTF8Encoding(false, true), true, 4096, true))
        {
            return reader.ReadToEnd();
        }
    }

    public void Revalidate()
    {
        if (directoryHandles.Count == 0 || directoryHandles.Count != directoryIdentities.Count) throw new InvalidOperationException("R2c-R audited script parent chain is not held");
        for (int index = 0; index < directoryHandles.Count; index++)
        {
            SafeFileHandle handle = directoryHandles[index];
            if (handle == null || handle.IsClosed) throw new InvalidOperationException("R2c-R audited script parent handle is not held");
            BY_HANDLE_FILE_INFORMATION information = AssertNonReparseDirectory(handle, "audited script parent");
            if (!Identity(information).Equals(directoryIdentities[index])) throw new InvalidOperationException("R2c-R audited script parent identity changed");
        }
        if (terminalHandle == null || terminalHandle.IsClosed || input == null) throw new InvalidOperationException("R2c-R audited script terminal handle is not held");
        BY_HANDLE_FILE_INFORMATION terminalInformation = AssertRegularNonReparseFile(terminalHandle, "held audited script terminal");
        if (!Identity(terminalInformation).Equals(terminalIdentity)) throw new InvalidOperationException("R2c-R audited script terminal identity changed");
        using (SafeFileHandle reopened = OpenRelative(directoryHandles[directoryHandles.Count - 1], terminalLeaf, false))
        {
            BY_HANDLE_FILE_INFORMATION reopenedInformation = AssertRegularNonReparseFile(reopened, "reopened audited script terminal");
            if (!Identity(reopenedInformation).Equals(terminalIdentity)) throw new InvalidOperationException("R2c-R audited script path no longer resolves to the audited object");
        }
    }

    public void ReleaseTerminalForGuardrail()
    {
        if (input != null) { input.Dispose(); input = null; }
        if (terminalHandle != null) { terminalHandle.Dispose(); terminalHandle = null; }
    }

    public void ReleaseAllHandlesForGuardrail()
    {
        ReleaseTerminalForGuardrail();
        DisposeDirectories();
    }

    public bool AllHandlesClosedForGuardrail
    {
        get
        {
            if (terminalHandle != null && !terminalHandle.IsClosed) return false;
            foreach (SafeFileHandle handle in directoryHandles) if (handle != null && !handle.IsClosed) return false;
            return true;
        }
    }

    public static void BeginGuardrailFault(string point)
    {
        if (guardrailFaultPoint != null || guardrailFaultHandle != null) throw new InvalidOperationException("R2c-R audited-script fault state was not clean");
        guardrailFaultPoint = point;
    }

    public static void EndGuardrailFault()
    {
        guardrailFaultPoint = null;
    }

    public static bool GuardrailFaultHandleIsClosed
    {
        get { return guardrailFaultHandle != null && guardrailFaultHandle.IsClosed; }
    }

    public static void ResetGuardrailFaultHandle()
    {
        SafeFileHandle captured = guardrailFaultHandle;
        guardrailFaultHandle = null;
        if (captured != null && !captured.IsClosed) captured.Dispose();
    }

    public static int GuardrailOpenCount { get { return guardrailOpenCount; } }
    public static int GuardrailDisposeCount { get { return guardrailDisposeCount; } }

    public static void ResetGuardrailOwnershipCounts()
    {
        System.Threading.Interlocked.Exchange(ref guardrailOpenCount, 0);
        System.Threading.Interlocked.Exchange(ref guardrailDisposeCount, 0);
    }

    private SafeFileHandle BindDirectoryChain(string path)
    {
        string volumeRoot = System.IO.Path.GetPathRoot(path);
        if (String.IsNullOrEmpty(volumeRoot) || !Regex.IsMatch(volumeRoot, @"\A[A-Za-z]:\\\z", RegexOptions.CultureInvariant)) throw new InvalidOperationException("R2c-R audited script tool root must use one local drive-letter volume");
        SafeFileHandle volume = CreateFileW(volumeRoot, FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero);
        if (volume.IsInvalid)
        {
            int error = Marshal.GetLastWin32Error();
            volume.Dispose();
            throw new Win32Exception(error, "Could not hold the R2c-R audited-script volume root");
        }
        try
        {
            HoldDirectory(volume, "audited-script volume root");
            ThrowIfGuardrailFault("parent-post-open", volume);
            volume = null;
        }
        finally { if (volume != null) volume.Dispose(); }
        SafeFileHandle parent = directoryHandles[directoryHandles.Count - 1];
        string relative = path.Substring(volumeRoot.Length);
        foreach (string component in relative.Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries))
        {
            SafeFileHandle child = OpenRelative(parent, component, true);
            try
            {
                HoldDirectory(child, "audited-script parent component");
                ScriptIdentity childIdentity = directoryIdentities[directoryIdentities.Count - 1];
                if (childIdentity.VolumeSerialNumber != directoryIdentities[0].VolumeSerialNumber) throw new InvalidOperationException("R2c-R audited script parent chain crossed its held volume");
                ThrowIfGuardrailFault("parent-post-open", child);
                parent = child;
                child = null;
            }
            finally { if (child != null) child.Dispose(); }
        }
        return parent;
    }

    private void HoldDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information = AssertNonReparseDirectory(handle, label);
        directoryHandles.Add(handle);
        try { directoryIdentities.Add(Identity(information)); }
        catch
        {
            directoryHandles.RemoveAt(directoryHandles.Count - 1);
            throw;
        }
    }

    private static SafeFileHandle OpenRelative(SafeFileHandle parent, string name, bool directory)
    {
        ValidateComponent(name);
        IntPtr nameBuffer = Marshal.StringToHGlobalUni(name);
        IntPtr nameStructure = IntPtr.Zero;
        IntPtr rawHandle = IntPtr.Zero;
        try
        {
            UNICODE_STRING unicodeName = new UNICODE_STRING();
            unicodeName.Length = checked((ushort)(name.Length * 2));
            unicodeName.MaximumLength = checked((ushort)((name.Length + 1) * 2));
            unicodeName.Buffer = nameBuffer;
            nameStructure = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
            Marshal.StructureToPtr(unicodeName, nameStructure, false);
            OBJECT_ATTRIBUTES attributes = new OBJECT_ATTRIBUTES();
            attributes.Length = (uint)Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES));
            attributes.RootDirectory = parent.DangerousGetHandle();
            attributes.ObjectName = nameStructure;
            if (!DirectoryIsCaseSensitive(parent)) attributes.Attributes = 0x00000040;
            IO_STATUS_BLOCK statusBlock;
            uint desiredAccess = (directory ? FILE_TRAVERSE : FILE_READ_DATA) | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
            uint shareAccess = directory ? FILE_SHARE_READ | FILE_SHARE_WRITE : FILE_SHARE_READ;
            uint createOptions = FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT | FILE_OPEN_REPARSE_POINT;
            if (directory) createOptions |= FILE_DIRECTORY_FILE;
            int status = NtCreateFile(out rawHandle, desiredAccess, ref attributes, out statusBlock, IntPtr.Zero, 0, shareAccess, 1, createOptions, IntPtr.Zero, 0);
            if (status < 0)
            {
                uint error = RtlNtStatusToDosError(status);
                throw new Win32Exception(unchecked((int)error), "Could not open the R2c-R audited script relative to its held parent; NTSTATUS=0x" + status.ToString("X8") + " Win32=" + error.ToString());
            }
            SafeFileHandle handle = new SafeFileHandle(rawHandle, true);
            rawHandle = IntPtr.Zero;
            return handle;
        }
        finally
        {
            if (rawHandle != IntPtr.Zero && rawHandle != new IntPtr(-1)) CloseHandle(rawHandle);
            if (nameStructure != IntPtr.Zero) Marshal.FreeHGlobal(nameStructure);
            Marshal.FreeHGlobal(nameBuffer);
        }
    }

    private static void ValidateComponent(string name)
    {
        if (String.IsNullOrEmpty(name) || name == "." || name == ".." || name.Length > 255 || name.IndexOfAny(new char[] { '\0', '\\', '/', ':' }) >= 0 || name[name.Length - 1] == '.' || name[name.Length - 1] == ' ') throw new InvalidOperationException("R2c-R audited script path component is not one normalized NT name");
    }

    private static bool DirectoryIsCaseSensitive(SafeFileHandle handle)
    {
        FILE_CASE_SENSITIVE_INFO information;
        if (!GetFileInformationByHandleEx(handle, 23, out information, (uint)Marshal.SizeOf(typeof(FILE_CASE_SENSITIVE_INFO)))) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not query the R2c-R audited-script parent case semantics");
        return (information.Flags & 1) != 0;
    }

    private static BY_HANDLE_FILE_INFORMATION AssertNonReparseDirectory(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (handle == null || handle.IsClosed || handle.IsInvalid || !GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) throw new InvalidOperationException("R2c-R " + label + " is not a directory");
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("R2c-R " + label + " is a reparse point");
        return information;
    }

    private static BY_HANDLE_FILE_INFORMATION AssertRegularNonReparseFile(SafeFileHandle handle, string label)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (handle == null || handle.IsClosed || handle.IsInvalid || !GetFileInformationByHandle(handle, out information)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inspect the R2c-R " + label);
        if ((information.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) throw new InvalidOperationException("R2c-R " + label + " is a reparse point");
        if ((information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0) throw new InvalidOperationException("R2c-R " + label + " is not a regular file");
        return information;
    }

    private static ScriptIdentity Identity(BY_HANDLE_FILE_INFORMATION information)
    {
        return new ScriptIdentity(information.VolumeSerialNumber, information.FileIndexHigh, information.FileIndexLow);
    }

    private static void ThrowIfGuardrailFault(string point, SafeFileHandle handle)
    {
        if (!String.Equals(guardrailFaultPoint, point, StringComparison.Ordinal)) return;
        guardrailFaultHandle = handle;
        throw new InvalidOperationException("R2c-R injected audited-script handle fault: " + point);
    }

    private void DisposeDirectories()
    {
        for (int index = directoryHandles.Count - 1; index >= 0; index--) directoryHandles[index].Dispose();
        directoryHandles.Clear();
        directoryIdentities.Clear();
    }

    public void Dispose()
    {
        if (disposedForGuardrail) return;
        disposedForGuardrail = true;
        ReleaseTerminalForGuardrail();
        DisposeDirectories();
        System.Threading.Interlocked.Increment(ref guardrailDisposeCount);
    }

    private struct ScriptIdentity : IEquatable<ScriptIdentity>
    {
        internal readonly uint VolumeSerialNumber;
        private readonly uint FileIndexHigh;
        private readonly uint FileIndexLow;
        internal ScriptIdentity(uint volumeSerialNumber, uint fileIndexHigh, uint fileIndexLow) { VolumeSerialNumber = volumeSerialNumber; FileIndexHigh = fileIndexHigh; FileIndexLow = fileIndexLow; }
        public bool Equals(ScriptIdentity other) { return VolumeSerialNumber == other.VolumeSerialNumber && FileIndexHigh == other.FileIndexHigh && FileIndexLow == other.FileIndexLow; }
        public override bool Equals(object value) { return value is ScriptIdentity && Equals((ScriptIdentity)value); }
        public override int GetHashCode() { return VolumeSerialNumber.GetHashCode() ^ FileIndexHigh.GetHashCode() ^ FileIndexLow.GetHashCode(); }
        public override string ToString() { return VolumeSerialNumber.ToString("X8") + ":" + FileIndexHigh.ToString("X8") + ":" + FileIndexLow.ToString("X8"); }
    }

    [StructLayout(LayoutKind.Sequential)] private struct FILETIME { public uint LowDateTime; public uint HighDateTime; }
    [StructLayout(LayoutKind.Sequential)] private struct BY_HANDLE_FILE_INFORMATION { public uint FileAttributes; public FILETIME CreationTime; public FILETIME LastAccessTime; public FILETIME LastWriteTime; public uint VolumeSerialNumber; public uint FileSizeHigh; public uint FileSizeLow; public uint NumberOfLinks; public uint FileIndexHigh; public uint FileIndexLow; }
    [StructLayout(LayoutKind.Sequential)] private struct UNICODE_STRING { public ushort Length; public ushort MaximumLength; public IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)] private struct OBJECT_ATTRIBUTES { public uint Length; public IntPtr RootDirectory; public IntPtr ObjectName; public uint Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }
    [StructLayout(LayoutKind.Sequential)] private struct IO_STATUS_BLOCK { public IntPtr Status; public UIntPtr Information; }
    [StructLayout(LayoutKind.Sequential)] private struct FILE_CASE_SENSITIVE_INFO { public uint Flags; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern SafeFileHandle CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandle(SafeFileHandle file, out BY_HANDLE_FILE_INFORMATION information);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetFileInformationByHandleEx(SafeFileHandle file, uint informationClass, out FILE_CASE_SENSITIVE_INFO information, uint bufferSize);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")] private static extern int NtCreateFile(out IntPtr fileHandle, uint desiredAccess, ref OBJECT_ATTRIBUTES objectAttributes, out IO_STATUS_BLOCK ioStatusBlock, IntPtr allocationSize, uint fileAttributes, uint shareAccess, uint createDisposition, uint createOptions, IntPtr eaBuffer, uint eaLength);
    [DllImport("ntdll.dll")] private static extern uint RtlNtStatusToDosError(int status);
}

public sealed class AmeR2cRProcessJob : IDisposable
{
    private SafeFileHandle jobHandle;
    private SafeFileHandle processHandle;
    private FileStream output;

    public AmeR2cRProcessJob()
    {
        IntPtr rawHandle = CreateJobObject(IntPtr.Zero, null);
        if (rawHandle == IntPtr.Zero || rawHandle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not create the R2c-R process job");
        jobHandle = new SafeFileHandle(rawHandle, true);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        limits.BasicLimitInformation.LimitFlags = 0x00002000;
        int length = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
        IntPtr buffer = Marshal.AllocHGlobal(length);
        try
        {
            Marshal.StructureToPtr(limits, buffer, false);
            if (!SetInformationJobObject(rawHandle, 9, buffer, (uint)length)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not configure the R2c-R process job");
        }
        catch { jobHandle.Dispose(); throw; }
        finally { Marshal.FreeHGlobal(buffer); }
    }

    public Process Start(string fileName, string[] arguments, string workingDirectory, string outputPath)
    {
        if (jobHandle == null || jobHandle.IsClosed) throw new ObjectDisposedException("AmeR2cRProcessJob");
        if (processHandle != null) throw new InvalidOperationException("The R2c-R process job already owns a process");
        output = new FileStream(outputPath, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.ReadWrite | FileShare.Delete, 4096, FileOptions.DeleteOnClose);
        IntPtr outputHandle = output.SafeFileHandle.DangerousGetHandle();
        if (!SetHandleInformation(outputHandle, 1, 1)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not inherit the R2c-R output handle");
        STARTUPINFO startup = new STARTUPINFO();
        startup.cb = (uint)Marshal.SizeOf(typeof(STARTUPINFO));
        startup.dwFlags = 0x00000100;
        startup.hStdInput = GetStdHandle(-10);
        startup.hStdOutput = outputHandle;
        startup.hStdError = outputHandle;
        PROCESS_INFORMATION process;
        StringBuilder commandLine = new StringBuilder(Quote(fileName));
        foreach (string argument in arguments) commandLine.Append(' ').Append(Quote(argument));
        if (!CreateProcess(fileName, commandLine, IntPtr.Zero, IntPtr.Zero, true, 0x00000004, IntPtr.Zero, workingDirectory, ref startup, out process)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not start the R2c-R owned process");
        try
        {
            if (!AssignProcessToJobObject(jobHandle.DangerousGetHandle(), process.hProcess))
            {
                int error = Marshal.GetLastWin32Error(); TerminateProcess(process.hProcess, 1); throw new Win32Exception(error, "Could not assign the R2c-R process to its job");
            }
            if (ResumeThread(process.hThread) == UInt32.MaxValue)
            {
                int error = Marshal.GetLastWin32Error(); TerminateProcess(process.hProcess, 1); throw new Win32Exception(error, "Could not resume the R2c-R owned process");
            }
            processHandle = new SafeFileHandle(process.hProcess, true);
            process.hProcess = IntPtr.Zero;
            return Process.GetProcessById((int)process.dwProcessId);
        }
        finally
        {
            CloseHandle(process.hThread);
            if (process.hProcess != IntPtr.Zero) CloseHandle(process.hProcess);
        }
    }

    public uint PrimaryExitCode
    {
        get
        {
            uint exitCode;
            if (processHandle == null || processHandle.IsClosed || !GetExitCodeProcess(processHandle.DangerousGetHandle(), out exitCode)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Could not read the R2c-R exit code");
            if (exitCode == 259) throw new InvalidOperationException("The R2c-R owned process is still active");
            return exitCode;
        }
    }

    public string[] ReadOutputLines()
    {
        if (output == null) return new string[0];
        output.Flush();
        output.Position = 0;
        List<string> lines = new List<string>();
        using (StreamReader reader = new StreamReader(output, Encoding.UTF8, true, 4096, true))
        {
            string line;
            while ((line = reader.ReadLine()) != null) lines.Add(line);
        }
        return lines.ToArray();
    }

    public void TerminateOwnedTree()
    {
        if (jobHandle != null) { jobHandle.Dispose(); jobHandle = null; }
    }

    private static string Quote(string value)
    {
        if (value.Length > 0 && value.IndexOfAny(new char[] { ' ', '\t', '\n', '\v', '"' }) < 0) return value;
        StringBuilder quoted = new StringBuilder("\"");
        int backslashes = 0;
        foreach (char current in value)
        {
            if (current == '\\') backslashes++;
            else if (current == '"') { quoted.Append('\\', backslashes * 2 + 1).Append(current); backslashes = 0; }
            else { quoted.Append('\\', backslashes).Append(current); backslashes = 0; }
        }
        return quoted.Append('\\', backslashes * 2).Append('"').ToString();
    }

    public void Dispose()
    {
        if (jobHandle != null) { jobHandle.Dispose(); jobHandle = null; }
        if (processHandle != null) { processHandle.Dispose(); processHandle = null; }
        if (output != null) { output.Dispose(); output = null; }
    }

    [StructLayout(LayoutKind.Sequential)] private struct JOBOBJECT_BASIC_LIMIT_INFORMATION { public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit; public uint LimitFlags; public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize; public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass; }
    [StructLayout(LayoutKind.Sequential)] private struct IO_COUNTERS { public ulong ReadOperationCount; public ulong WriteOperationCount; public ulong OtherOperationCount; public ulong ReadTransferCount; public ulong WriteTransferCount; public ulong OtherTransferCount; }
    [StructLayout(LayoutKind.Sequential)] private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION { public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation; public IO_COUNTERS IoInfo; public UIntPtr ProcessMemoryLimit; public UIntPtr JobMemoryLimit; public UIntPtr PeakProcessMemoryUsed; public UIntPtr PeakJobMemoryUsed; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct STARTUPINFO { public uint cb; public string lpReserved; public string lpDesktop; public string lpTitle; public uint dwX; public uint dwY; public uint dwXSize; public uint dwYSize; public uint dwXCountChars; public uint dwYCountChars; public uint dwFillAttribute; public uint dwFlags; public ushort wShowWindow; public ushort cbReserved2; public IntPtr lpReserved2; public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError; }
    [StructLayout(LayoutKind.Sequential)] private struct PROCESS_INFORMATION { public IntPtr hProcess; public IntPtr hThread; public uint dwProcessId; public uint dwThreadId; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern IntPtr CreateJobObject(IntPtr securityAttributes, string name);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetInformationJobObject(IntPtr job, uint informationClass, IntPtr information, uint informationLength);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool CreateProcess(string applicationName, StringBuilder commandLine, IntPtr processAttributes, IntPtr threadAttributes, bool inheritHandles, uint creationFlags, IntPtr environment, string currentDirectory, ref STARTUPINFO startupInformation, out PROCESS_INFORMATION processInformation);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool TerminateProcess(IntPtr process, uint exitCode);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool SetHandleInformation(IntPtr handle, uint mask, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GetStdHandle(int standardHandle);
}
'@
        if ($ForceCompilerFailure) {
            $nativeTypeDefinition += "`npublic class AmeR2cRForcedCompilerFailure {"
        }
        Add-Type -TypeDefinition $nativeTypeDefinition
        Assert-AmeR2cRCompilerBootstrapHeld -Bootstrap $bootstrap
        [AmeR2cRNative]::AssertDirectoryAnchor([string]$bootstrap.Path)
    } finally {
        try {
            if ($environmentCaptured -and $environmentMutated) {
                try {
                    [Environment]::SetEnvironmentVariable("TEMP", $previousTemp, "Process")
                } finally {
                    [Environment]::SetEnvironmentVariable("TMP", $previousTmp, "Process")
                }
            }
        } finally {
            if ($null -ne $bootstrap) {
                Remove-AmeR2cRCompilerBootstrap -Bootstrap $bootstrap
            }
        }
    }
}

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

function Remove-AmeR2cRDisposableRoot {
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
    $rootPath = [string]$Fixture.Path
    $rootIdentity = [string]$Fixture.RootIdentityToken
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
            $entries = @([IO.DirectoryInfo]::new($rootPath).EnumerateFileSystemInfos())
            if ($entries.Count -ne 0) {
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
            $Fixture.Dispose()
        }
    }
    if ($null -ne $cleanupFailure) {
        throw (
            "R2c-R safe cleanup failed and retained the owned root for investigation; " +
            "no path fallback was attempted; owned_leaf=$($Fixture.RootName); " +
            "expected_identity=$rootIdentity; original_path=$rootPath; " +
            $cleanupFailure.Exception.Message
        )
    }
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

function Invoke-AmeR2cROwnedProcess {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("CargoExactTest", "PowerShell")]
        [string]$ExecutableKind,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [AllowNull()][string]$OutputPath,
        [ValidateRange(1, 3600000)][int]$TimeoutMilliseconds = 300000,
        [switch]$Detached
    )
    $auditState = $null
    $boundaryState = $null
    $job = $null
    $process = $null
    $timedOut = $false
    $processId = 0
    $exitCode = -1
    $lines = @()
    try {
        $expectedWorkingDirectory = [IO.Path]::GetFullPath(
            (Join-Path $PSScriptRoot "..")
        ).TrimEnd('\')
        $actualWorkingDirectory = [IO.Path]::GetFullPath($WorkingDirectory).TrimEnd('\')
        if (-not $actualWorkingDirectory.Equals(
                $expectedWorkingDirectory,
                [StringComparison]::OrdinalIgnoreCase
            )) {
            throw "R2c-R verified process boundary rejected its working directory"
        }
        $boundaryPath = Join-Path `
            $PSScriptRoot `
            "acceptance_r2c_change_driven_reliability_common.ps1"
        $boundaryState = Assert-AmeR2cRDeletionSafetySource `
            -SourceText $null `
            -SourcePath $boundaryPath `
            -Label "verified process boundary" `
            -RetainSnapshots
        if ($ExecutableKind -ceq "CargoExactTest") {
            if ($Detached) {
                throw "R2c-R verified process boundary cannot detach a Cargo test"
            }
            $cargoFallback = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
            if ([IO.File]::Exists($cargoFallback)) {
                $expectedExecutable = [IO.Path]::GetFullPath($cargoFallback)
            } else {
                $cargoCommand = Microsoft.PowerShell.Core\Get-Command `
                    "cargo" `
                    -CommandType Application `
                    -ErrorAction SilentlyContinue |
                    Microsoft.PowerShell.Utility\Select-Object -First 1
                if ($null -eq $cargoCommand) {
                    throw "R2c-R verified process boundary could not resolve Cargo"
                }
                $expectedExecutable = [IO.Path]::GetFullPath($cargoCommand.Source)
            }
            $actualExecutable = [IO.Path]::GetFullPath($FileName)
            if (-not $actualExecutable.Equals(
                    $expectedExecutable,
                    [StringComparison]::OrdinalIgnoreCase
                )) {
                throw "R2c-R verified process boundary rejected the Cargo executable"
            }
            $caseMatches = 0
            foreach ($case in @(Get-AmeR2cRCaseDefinitions)) {
                $expectedArguments = @(Get-AmeR2cRCaseArguments -Case $case)
                if ($expectedArguments.Count -ne $Arguments.Count) { continue }
                $matches = $true
                for ($index = 0; $index -lt $Arguments.Count; $index += 1) {
                    if ([string]$Arguments[$index] -cne [string]$expectedArguments[$index]) {
                        $matches = $false
                        break
                    }
                }
                if ($matches) { $caseMatches += 1 }
            }
            if ($caseMatches -ne 1) {
                throw "R2c-R verified process boundary rejected a non-matrix Cargo command"
            }
        } else {
            $currentProcess = Microsoft.PowerShell.Management\Get-Process -Id $PID
            try { $expectedExecutable = [IO.Path]::GetFullPath($currentProcess.Path) }
            finally { $currentProcess.Dispose() }
            $actualExecutable = [IO.Path]::GetFullPath($FileName)
            if (-not $actualExecutable.Equals(
                    $expectedExecutable,
                    [StringComparison]::OrdinalIgnoreCase
                )) {
                throw "R2c-R verified process boundary rejected the PowerShell executable"
            }
            if ($Arguments.Count -lt 4 -or
                $Arguments[0] -cne "-NoProfile" -or
                $Arguments[1] -cne "-NonInteractive") {
                throw "R2c-R verified process boundary rejected PowerShell host arguments"
            }
            $payloadLabel = "PowerShell $($Arguments[2]) payload"
            if ($Arguments[2] -ceq "-Command") {
                if ($Arguments.Count -ne 4) {
                    throw "R2c-R verified process boundary rejected ambiguous Command arguments"
                }
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText ([string]$Arguments[3]) `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } elseif ($Arguments[2] -ceq "-EncodedCommand") {
                if ($Arguments.Count -ne 4 -or
                    [string]$Arguments[3] -notmatch '^[A-Za-z0-9+/]+={0,2}$') {
                    throw "R2c-R verified process boundary rejected malformed EncodedCommand arguments"
                }
                try {
                    $payloadBytes = [Convert]::FromBase64String([string]$Arguments[3])
                } catch {
                    throw "R2c-R verified process boundary rejected malformed EncodedCommand payload"
                }
                if (($payloadBytes.Length % 2) -ne 0 -or
                    [Convert]::ToBase64String($payloadBytes) -cne [string]$Arguments[3]) {
                    throw "R2c-R verified process boundary rejected non-canonical EncodedCommand payload"
                }
                $payloadText = [Text.Encoding]::Unicode.GetString($payloadBytes)
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText $payloadText `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } elseif ($Arguments[2] -ceq "-File") {
                if ($Detached -or $Arguments.Count -lt 4) {
                    throw "R2c-R verified process boundary rejected detached or missing File payload"
                }
                $payloadPath = [IO.Path]::GetFullPath([string]$Arguments[3])
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText $null `
                    -SourcePath $payloadPath `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } else {
                throw "R2c-R verified process boundary rejected an unknown PowerShell selector"
            }
        }
        if ($Detached) {
            if ($ExecutableKind -cne "PowerShell" -or $Arguments[2] -ceq "-File") {
                throw "R2c-R verified process boundary rejected an unsafe detached payload"
            }
            if ($null -ne $auditState) {
                Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $auditState
            }
            Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $boundaryState
            $process = Microsoft.PowerShell.Management\Start-Process `
                -FilePath $FileName `
                -ArgumentList $Arguments `
                -WorkingDirectory $actualWorkingDirectory `
                -WindowStyle Hidden `
                -PassThru
            $processId = $process.Id
            return [pscustomobject]@{
                ExitCode = -1
                Lines = @()
                Process = $process
                ProcessId = $processId
                TimedOut = $false
            }
        }
        if ([string]::IsNullOrEmpty($OutputPath)) {
            throw "R2c-R verified process boundary requires an owned output path"
        }
        $job = [AmeR2cRProcessJob]::new()
        if ($null -ne $auditState) {
            Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $auditState
        }
        Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $boundaryState
        $process = $job.Start($FileName, $Arguments, $WorkingDirectory, $OutputPath)
        $processId = $process.Id
        if (-not $process.WaitForExit($TimeoutMilliseconds)) {
            $timedOut = $true
            $job.TerminateOwnedTree()
            if (-not $process.WaitForExit(5000)) { throw "R2c-R timed-out process tree did not terminate after job disposal" }
        } else { $exitCode = [int]$job.PrimaryExitCode }
        $lines = @($job.ReadOutputLines())
    } finally {
        if ($null -ne $job) { $job.Dispose() }
        if ($null -ne $process -and -not $Detached) { $process.Dispose() }
        if ($null -ne $auditState) {
            Close-AmeR2cRDeletionAuditState -State $auditState
        }
        if ($null -ne $boundaryState) {
            Close-AmeR2cRDeletionAuditState -State $boundaryState
        }
    }
    return [pscustomobject]@{ ExitCode = $exitCode; Lines = $lines; ProcessId = $processId; TimedOut = $timedOut }
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

function Get-AmeR2cRTextSha256 {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text.Replace("`r`n", "`n"))
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes)) -replace '-', '').ToLowerInvariant()
    } finally { $algorithm.Dispose() }
}

function Add-AmeR2cRDeletionAuditBudgetValue {
    param(
        [Parameter(Mandatory = $true)][string]$Key,
        [Parameter(Mandatory = $true)][uint64]$Current,
        [Parameter(Mandatory = $true)][uint64]$Delta,
        [Parameter(Mandatory = $true)][uint64]$Limit
    )
    if ($Delta -gt ([uint64]::MaxValue - $Current)) {
        throw "R2c-R deletion safety budget exceeded; budget=$Key limit=$Limit actual=overflow"
    }
    $actual = [uint64]($Current + $Delta)
    if ($actual -gt $Limit) {
        throw "R2c-R deletion safety budget exceeded; budget=$Key limit=$Limit actual=$actual"
    }
    return $actual
}

function Assert-AmeR2cRDeletionAuditBudgetValue {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][string]$Key,
        [Parameter(Mandatory = $true)][uint64]$Current,
        [Parameter(Mandatory = $true)][uint64]$Delta
    )
    if (-not $State.BudgetLimits.Contains($Key)) {
        throw "R2c-R deletion safety budget key is unknown: $Key"
    }
    return Add-AmeR2cRDeletionAuditBudgetValue `
        -Key $Key `
        -Current $Current `
        -Delta $Delta `
        -Limit ([uint64]$State.BudgetLimits[$Key])
}

function New-AmeR2cRDeletionAuditState {
    param([AllowNull()][hashtable]$BudgetOverridesForGuardrail)
    $limits = [ordered]@{}
    foreach ($key in $script:AmeR2cRDeletionAuditBudgetLimits.Keys) {
        $limits[$key] = [uint64]$script:AmeR2cRDeletionAuditBudgetLimits[$key]
    }
    if ($null -ne $BudgetOverridesForGuardrail) {
        foreach ($key in $BudgetOverridesForGuardrail.Keys) {
            if (-not $limits.Contains($key)) {
                throw "R2c-R deletion safety budget override key is unknown: $key"
            }
            $limits[$key] = [uint64]$BudgetOverridesForGuardrail[$key]
        }
    }
    return [pscustomobject]@{
        AuditedScopes = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::Ordinal
        )
        BudgetActual = [ordered]@{
            "source-count" = [uint64]0
            "total-bytes" = [uint64]0
            "ast-nodes" = [uint64]0
            "function-count" = [uint64]0
            "scope-count" = [uint64]0
            "queue-count" = [uint64]0
        }
        BudgetVisitedHighWater = [ordered]@{
            "ast-nodes" = [uint64]0
        }
        BudgetLimits = $limits
        Functions = @{}
        HeldStreams = [Collections.Generic.List[IDisposable]]::new()
        Queue = [Collections.Generic.Queue[object]]::new()
        SourceByIdentity = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        SourceByKey = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        SourceByPath = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        Sources = [Collections.Generic.List[object]]::new()
        ToolRoot = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
    }
}

function Close-AmeR2cRDeletionAuditState {
    param([Parameter(Mandatory = $true)][psobject]$State)
    for ($index = $State.HeldStreams.Count - 1; $index -ge 0; $index -= 1) {
        $State.HeldStreams[$index].Dispose()
    }
    $State.HeldStreams.Clear()
}

function Assert-AmeR2cRDeletionAuditSnapshotsHeld {
    param([Parameter(Mandatory = $true)][psobject]$State)
    foreach ($source in $State.Sources) {
        if ($null -ne $source.Snapshot) {
            $source.Snapshot.Revalidate()
        }
    }
}

function Read-AmeR2cRAuditedScriptSnapshot {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $fullPath = [IO.Path]::GetFullPath($Path)
    if ([IO.Path]::GetExtension($fullPath) -cne ".ps1") {
        throw "R2c-R deletion safety audit rejected a source without the exact .ps1 extension"
    }
    if ($null -eq ("AmeR2cRAuditedScriptSnapshot" -as [type])) {
        throw "R2c-R native audited-script identity types are not initialized"
    }
    $snapshot = [AmeR2cRAuditedScriptSnapshot]::Open($State.ToolRoot, $fullPath)
    try {
        $byteLength = [uint64]$snapshot.Length
        Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "per-source-bytes" `
            -Current 0 `
            -Delta $byteLength | Out-Null
        $text = $snapshot.ReadUtf8Text()
        $result = [pscustomobject]@{
            ByteLength = $byteLength
            Digest = Get-AmeR2cRTextSha256 -Text $text
            Identity = [string]$snapshot.IdentityToken
            NativeSnapshot = $snapshot
            Path = $fullPath
            Text = $text
        }
        $snapshot = $null
        return $result
    } finally {
        if ($null -ne $snapshot) { $snapshot.Dispose() }
    }
}

function Get-AmeR2cRAstOwningFunction {
    param([Parameter(Mandatory = $true)][Management.Automation.Language.Ast]$Node)
    $current = $Node.Parent
    while ($null -ne $current) {
        if ($current -is [Management.Automation.Language.FunctionDefinitionAst]) {
            return $current
        }
        $current = $current.Parent
    }
    return $null
}

function Resolve-AmeR2cRAuditDotSourcePath {
    param(
        [Parameter(Mandatory = $true)]
        [Management.Automation.Language.CommandAst]$Command,
        [AllowNull()][string]$SourcePath
    )
    $target = $Command.CommandElements[0]
    $candidate = $null
    if ($target -is [Management.Automation.Language.StringConstantExpressionAst]) {
        $candidate = [string]$target.Value
    } elseif ($target -is [Management.Automation.Language.ParenExpressionAst] -and
        $null -ne $SourcePath) {
        $pipeline = $target.Pipeline
        if ($pipeline.PipelineElements.Count -eq 1 -and
            $pipeline.PipelineElements[0] -is [Management.Automation.Language.CommandAst]) {
            $join = $pipeline.PipelineElements[0]
            if ($join.GetCommandName() -ceq "Join-Path" -and
                $join.CommandElements.Count -eq 3 -and
                $join.CommandElements[1] -is [Management.Automation.Language.VariableExpressionAst] -and
                $join.CommandElements[1].VariablePath.UserPath -ceq "PSScriptRoot" -and
                $join.CommandElements[2] -is [Management.Automation.Language.StringConstantExpressionAst]) {
                $leaf = [string]$join.CommandElements[2].Value
                if ([IO.Path]::GetFileName($leaf) -cne $leaf) {
                    throw "R2c-R deletion safety audit rejected non-leaf PSScriptRoot loader"
                }
                $candidate = Join-Path ([IO.Path]::GetDirectoryName($SourcePath)) $leaf
            }
        }
    }
    if ([string]::IsNullOrEmpty($candidate)) {
        throw "R2c-R deletion safety audit rejected unresolved dot-source"
    }
    if (-not [IO.Path]::IsPathRooted($candidate)) {
        if ($null -eq $SourcePath) {
            throw "R2c-R deletion safety audit rejected unresolved relative dot-source"
        }
        $candidate = Join-Path ([IO.Path]::GetDirectoryName($SourcePath)) $candidate
    }
    return [IO.Path]::GetFullPath($candidate)
}

function Add-AmeR2cRDeletionAuditAstNodesBounded {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][Management.Automation.Language.Ast]$Ast
    )
    $counter = [pscustomobject]@{
        Current = [uint64]$State.BudgetActual["ast-nodes"]
    }
    $null = $Ast.FindAll({
        param($node)
        $actual = [uint64]($counter.Current + 1)
        if ($actual -gt [uint64]$State.BudgetVisitedHighWater["ast-nodes"]) {
            $State.BudgetVisitedHighWater["ast-nodes"] = $actual
        }
        $counter.Current = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "ast-nodes" `
            -Current ([uint64]$counter.Current) `
            -Delta 1
        $State.BudgetActual["ast-nodes"] = [uint64]$counter.Current
        return $false
    }, $true)
}

function Add-AmeR2cRDeletionAuditSource {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [AllowNull()][object]$SourceText,
        [Parameter(Mandatory = $true)][string]$Label,
        [AllowNull()][string]$SourcePath,
        [Parameter(Mandatory = $true)][bool]$IsRoot,
        [Parameter(Mandatory = $true)][uint64]$Depth
    )
    if ($null -ne $SourceText -and $SourceText -isnot [string]) {
        throw "R2c-R deletion safety audit source text must be a string"
    }
    Assert-AmeR2cRDeletionAuditBudgetValue `
        -State $State `
        -Key "dot-source-depth" `
        -Current 0 `
        -Delta $Depth | Out-Null
    $newSourceCount = Assert-AmeR2cRDeletionAuditBudgetValue `
        -State $State `
        -Key "source-count" `
        -Current ([uint64]$State.BudgetActual["source-count"]) `
        -Delta 1
    $State.BudgetActual["source-count"] = $newSourceCount
    $snapshot = $null
    $nativeSnapshot = $null
    $pathKey = $null
    $identity = $null
    try {
        if (-not [string]::IsNullOrEmpty($SourcePath)) {
            $snapshot = Read-AmeR2cRAuditedScriptSnapshot -State $State -Path $SourcePath
            $nativeSnapshot = $snapshot.NativeSnapshot
            if ($null -ne $SourceText -and $SourceText -cne $snapshot.Text) {
                throw "R2c-R deletion safety audit source text changed before identity binding"
            }
            $SourceText = [string]$snapshot.Text
            $SourcePath = [string]$snapshot.Path
            $pathKey = $SourcePath
            $byteLength = [uint64]$snapshot.ByteLength
            $sourceDigest = [string]$snapshot.Digest
            $identity = [string]$snapshot.Identity
            if ($State.SourceByIdentity.ContainsKey($identity)) {
                $existing = $State.SourceByIdentity[$identity]
                if ($existing.Digest -cne $sourceDigest -or $existing.Text -cne $SourceText) {
                    throw "R2c-R deletion safety audit held identity produced conflicting source bytes"
                }
                $State.SourceByPath[$pathKey] = $existing
                return $existing
            }
            $key = "file:${identity}:$sourceDigest"
            $newTotal = Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "total-bytes" `
                -Current ([uint64]$State.BudgetActual["total-bytes"]) `
                -Delta $byteLength
            $State.BudgetActual["total-bytes"] = $newTotal
        } else {
            if ($null -eq $SourceText) {
                throw "R2c-R deletion safety audit received no source text"
            }
            $byteLength = [uint64][Text.Encoding]::UTF8.GetByteCount($SourceText)
            Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "per-source-bytes" `
                -Current 0 `
                -Delta $byteLength | Out-Null
            $newTotal = Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "total-bytes" `
                -Current ([uint64]$State.BudgetActual["total-bytes"]) `
                -Delta $byteLength
            $State.BudgetActual["total-bytes"] = $newTotal
            $sourceDigest = Get-AmeR2cRTextSha256 -Text $SourceText
            $key = "memory:${Label}:$sourceDigest"
        }
        if ($State.SourceByKey.ContainsKey($key)) {
            return $State.SourceByKey[$key]
        }
        $tokens = $null
        $parseErrors = $null
        $ast = [Management.Automation.Language.Parser]::ParseInput(
            $SourceText,
            [ref]$tokens,
            [ref]$parseErrors
        )
        if ($parseErrors.Count -ne 0) {
            throw "R2c-R deletion safety audit could not parse '$Label'"
        }
        Add-AmeR2cRDeletionAuditAstNodesBounded -State $State -Ast $ast
        $functions = @($ast.FindAll({
            param($node)
            $node -is [Management.Automation.Language.FunctionDefinitionAst]
        }, $true))
        $newFunctionCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "function-count" `
            -Current ([uint64]$State.BudgetActual["function-count"]) `
            -Delta ([uint64]$functions.Count)
        $State.BudgetActual["function-count"] = $newFunctionCount
        $record = [pscustomobject]@{
            Ast = $ast
            ByteLength = $byteLength
            Depth = $Depth
            Digest = $sourceDigest
            IsRoot = $IsRoot
            Identity = $identity
            Key = $key
            Label = $Label
            Path = $SourcePath
            Snapshot = $nativeSnapshot
            Text = $SourceText
        }
        if ($null -ne $nativeSnapshot) {
            $State.HeldStreams.Add($nativeSnapshot)
            $nativeSnapshot = $null
        }
        $State.SourceByKey[$key] = $record
        if ($null -ne $identity) { $State.SourceByIdentity[$identity] = $record }
        if ($null -ne $pathKey) { $State.SourceByPath[$pathKey] = $record }
        $State.Sources.Add($record)
        foreach ($function in $functions) {
            if (-not $State.Functions.ContainsKey($function.Name)) {
                $State.Functions[$function.Name] = [Collections.Generic.List[object]]::new()
            }
            $State.Functions[$function.Name].Add([pscustomobject]@{
                Ast = $function
                Record = $record
            })
        }
        return $record
    } finally {
        if ($null -ne $nativeSnapshot) { $nativeSnapshot.Dispose() }
    }
}

function Add-AmeR2cRDeletionAuditScope {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][psobject]$Record,
        [AllowNull()][Management.Automation.Language.FunctionDefinitionAst]$Function
    )
    $scopeKey = if ($null -eq $Function) {
        "$($Record.Key):top"
    } else {
        "$($Record.Key):function:$($Function.Name):$($Function.Extent.StartOffset)"
    }
    if (-not $State.AuditedScopes.Contains($scopeKey)) {
        $newScopeCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "scope-count" `
            -Current ([uint64]$State.BudgetActual["scope-count"]) `
            -Delta 1
        $newQueueCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "queue-count" `
            -Current ([uint64]$State.Queue.Count) `
            -Delta 1
        $State.AuditedScopes.Add($scopeKey) | Out-Null
        $State.Queue.Enqueue([pscustomobject]@{
            Function = $Function
            Key = $scopeKey
            Record = $Record
        })
        $State.BudgetActual["scope-count"] = $newScopeCount
        if ($newQueueCount -gt [uint64]$State.BudgetActual["queue-count"]) {
            $State.BudgetActual["queue-count"] = $newQueueCount
        }
    }
}

function Assert-AmeR2cRProcessBoundaryContract {
    param([Parameter(Mandatory = $true)][psobject]$State)
    if (-not $State.Functions.ContainsKey("Invoke-AmeR2cROwnedProcess")) { return }
    $matches = @($State.Functions["Invoke-AmeR2cROwnedProcess"])
    if ($matches.Count -ne 1) {
        throw "R2c-R process execution boundary was ambiguous"
    }
    $actual = Get-AmeR2cRTextSha256 -Text $matches[0].Ast.Extent.Text
    if ($actual -cne $script:AmeR2cRVerifiedProcessBoundaryDigest) {
        throw (
            "R2c-R process boundary item Invoke-AmeR2cROwnedProcess changed; " +
            "expected=$script:AmeR2cRVerifiedProcessBoundaryDigest actual=$actual"
        )
    }
    $nativeDefinitions = @(
        foreach ($record in $State.Sources) {
            $assignments = @($record.Ast.FindAll({
                param($node)
                    $node -is [Management.Automation.Language.AssignmentStatementAst] -and
                    $node.Left -is [Management.Automation.Language.VariableExpressionAst] -and
                    $node.Left.VariablePath.UserPath -ceq "nativeTypeDefinition" -and
                    $node.Right -is [Management.Automation.Language.CommandExpressionAst] -and
                    $node.Right.Expression -is
                        [Management.Automation.Language.StringConstantExpressionAst] -and
                    $node.Right.Expression.Value.IndexOf(
                        "public sealed class AmeR2cRProcessJob",
                        [StringComparison]::Ordinal
                    ) -ge 0
            }, $true))
            foreach ($assignment in $assignments) {
                [string]$assignment.Right.Expression.Value
            }
        }
    )
    if ($nativeDefinitions.Count -ne 1) {
        throw "R2c-R native execution definition was missing or ambiguous"
    }
    $definitionActual = Get-AmeR2cRTextSha256 -Text $nativeDefinitions[0]
    if ($definitionActual -cne $script:AmeR2cRNativeDefinitionDigest) {
        throw (
            "R2c-R native boundary item nativeTypeDefinition changed; " +
            "expected=$script:AmeR2cRNativeDefinitionDigest actual=$definitionActual"
        )
    }
    $snapshotClasses = @(
        foreach ($value in $nativeDefinitions) {
            $snapshotStart = $value.IndexOf(
                "public sealed class AmeR2cRAuditedScriptSnapshot",
                [StringComparison]::Ordinal
            )
            $processStart = $value.IndexOf(
                "public sealed class AmeR2cRProcessJob",
                [StringComparison]::Ordinal
            )
            if ($snapshotStart -lt 0 -or $processStart -le $snapshotStart) {
                throw "R2c-R native audited-script identity boundary was missing"
            }
            $value.Substring($snapshotStart, $processStart - $snapshotStart)
        }
    )
    if ($snapshotClasses.Count -ne 1) {
        throw "R2c-R native audited-script identity boundary was ambiguous"
    }
    $snapshotActual = Get-AmeR2cRTextSha256 -Text $snapshotClasses[0]
    if ($snapshotActual -cne $script:AmeR2cRAuditedScriptSnapshotDigest) {
        throw (
            "R2c-R native boundary item AmeR2cRAuditedScriptSnapshot changed; " +
            "expected=$script:AmeR2cRAuditedScriptSnapshotDigest actual=$snapshotActual"
        )
    }
    $nativeClasses = @(
        foreach ($value in $nativeDefinitions) {
            $start = $value.IndexOf(
                "public sealed class AmeR2cRProcessJob",
                [StringComparison]::Ordinal
            )
            $value.Substring($start)
        }
    )
    if ($nativeClasses.Count -ne 1) {
        throw "R2c-R native process execution boundary was missing or ambiguous"
    }
    $nativeActual = Get-AmeR2cRTextSha256 -Text $nativeClasses[0]
    if ($nativeActual -cne $script:AmeR2cRNativeProcessBoundaryDigest) {
        throw (
            "R2c-R native process boundary item AmeR2cRProcessJob changed; " +
            "expected=$script:AmeR2cRNativeProcessBoundaryDigest actual=$nativeActual"
        )
    }
}

function Invoke-AmeR2cRDeletionAuditClosure {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][psobject]$Root
    )
    Add-AmeR2cRDeletionAuditScope -State $State -Record $Root -Function $null
    foreach ($function in @($Root.Ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst]
    }, $true))) {
        Add-AmeR2cRDeletionAuditScope -State $State -Record $Root -Function $function
    }
    while ($State.Queue.Count -gt 0) {
        $scope = $State.Queue.Dequeue()
        $ownerName = if ($null -eq $scope.Function) { "<top>" } else { $scope.Function.Name }
        $scopeAst = if ($null -eq $scope.Function) { $scope.Record.Ast } else { $scope.Function.Body }
        $commands = @($scopeAst.FindAll({
            param($node)
            if ($node -isnot [Management.Automation.Language.CommandAst]) { return $false }
            if ($null -ne $scope.Function) { return $true }
            return $null -eq (Get-AmeR2cRAstOwningFunction -Node $node)
        }, $true))
        foreach ($command in $commands) {
            if ($command.InvocationOperator -eq [Management.Automation.Language.TokenKind]::Dot) {
                $dependencyDepth = Assert-AmeR2cRDeletionAuditBudgetValue `
                    -State $State `
                    -Key "dot-source-depth" `
                    -Current ([uint64]$scope.Record.Depth) `
                    -Delta 1
                $dotPath = Resolve-AmeR2cRAuditDotSourcePath `
                    -Command $command `
                    -SourcePath $scope.Record.Path
                $dependency = Add-AmeR2cRDeletionAuditSource `
                    -State $State `
                    -SourceText $null `
                    -Label "dot-source:$dotPath" `
                    -SourcePath $dotPath `
                    -IsRoot $false `
                    -Depth $dependencyDepth
                Add-AmeR2cRDeletionAuditScope `
                    -State $State `
                    -Record $dependency `
                    -Function $null
                continue
            }
            $commandName = $command.GetCommandName()
            if ($command.InvocationOperator -ne
                    [Management.Automation.Language.TokenKind]::Unknown -or
                [string]::IsNullOrEmpty($commandName)) {
                throw (
                    "R2c-R deletion safety audit rejected unresolved command invocation " +
                    "in '$($scope.Record.Label)' scope '$ownerName'"
                )
            }
            $leafName = [string]$commandName
            $moduleName = $null
            $separatorIndex = $leafName.LastIndexOf('\', [StringComparison]::Ordinal)
            if ($separatorIndex -ge 0) {
                $moduleName = $leafName.Substring(0, $separatorIndex)
                $leafName = $leafName.Substring($separatorIndex + 1)
            }
            if ($leafName -iin @(
                "Remove-Item", "ri", "rm", "del", "erase", "rmdir", "rd",
                "Set-Alias", "sal", "New-Alias", "nal", "Import-Alias", "ipal",
                "Invoke-Expression", "iex", "Invoke-Command"
            )) {
                throw "R2c-R deletion safety audit rejected command '$leafName'"
            }
            if ($State.Functions.ContainsKey($leafName)) {
                $matches = @($State.Functions[$leafName])
                if ($matches.Count -ne 1) {
                    throw "R2c-R deletion safety audit rejected ambiguous helper '$leafName'"
                }
                Add-AmeR2cRDeletionAuditScope `
                    -State $State `
                    -Record $matches[0].Record `
                    -Function $matches[0].Ast
                continue
            }
            if ($leafName -ceq "Add-Type") {
                if ($ownerName -cne "Initialize-AmeR2cRNativeTypes" -or
                    $command.CommandElements.Count -ne 3 -or
                    $command.CommandElements[1] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[1].ParameterName -cne "TypeDefinition" -or
                    $command.CommandElements[2] -isnot
                        [Management.Automation.Language.VariableExpressionAst] -or
                    $command.CommandElements[2].VariablePath.UserPath -cne
                        "nativeTypeDefinition") {
                    throw "R2c-R deletion safety audit rejected Add-Type outside its exact native boundary"
                }
            }
            if ($leafName -ceq "Move-Item") {
                if ($command.CommandElements.Count -ne 5 -or
                    $command.CommandElements[1] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[1].ParameterName -cne "LiteralPath" -or
                    $command.CommandElements[2] -isnot
                        [Management.Automation.Language.StringConstantExpressionAst] -or
                    $command.CommandElements[3] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[3].ParameterName -cne "Destination" -or
                    $command.CommandElements[4] -isnot
                        [Management.Automation.Language.StringConstantExpressionAst]) {
                    throw "R2c-R deletion safety audit rejected non-literal Move-Item"
                }
                $moveSource = [IO.Path]::GetFullPath(
                    [string]$command.CommandElements[2].Value
                )
                $moveDestination = [IO.Path]::GetFullPath(
                    [string]$command.CommandElements[4].Value
                )
                $moveParent = [IO.Path]::GetDirectoryName($moveSource).TrimEnd('\')
                $moveLeaf = [IO.Path]::GetFileName($moveSource)
                if (-not $moveParent.Equals(
                        $State.ToolRoot,
                        [StringComparison]::OrdinalIgnoreCase
                    ) -or
                    $moveLeaf -cnotmatch '^\.ame-r2c-r-bootstrap-[0-9a-f]{32}$' -or
                    $moveDestination -cne ($moveSource + "-moved")) {
                    throw "R2c-R deletion safety audit rejected Move-Item outside the compiler bootstrap contract"
                }
            }
            if ($leafName -ceq "New-Item" -and
                @($command.CommandElements | Where-Object {
                    $_ -is [Management.Automation.Language.CommandParameterAst] -and
                        $_.ParameterName -ceq "Force"
                }).Count -ne 0) {
                throw "R2c-R deletion safety audit rejected forceful New-Item"
            }
            if (-not $script:AmeR2cRAllowedCmdlets.ContainsKey($leafName)) {
                throw (
                    "R2c-R deletion safety audit rejected unresolved helper or external " +
                    "executable '$leafName' in '$($scope.Record.Label)'"
                )
            }
            $expectedModule = [string]$script:AmeR2cRAllowedCmdlets[$leafName]
            if ($null -ne $moduleName -and $moduleName -cne $expectedModule) {
                throw (
                    "R2c-R deletion safety audit rejected module '$moduleName' for " +
                    "cmdlet '$leafName'; expected '$expectedModule'"
                )
            }
            if ($leafName -ceq "Start-Process" -and
                $ownerName -cne "Invoke-AmeR2cROwnedProcess") {
                throw "R2c-R deletion safety audit rejected Start-Process outside the verified boundary"
            }
        }

        $members = @($scopeAst.FindAll({
            param($node)
            if ($node -isnot [Management.Automation.Language.InvokeMemberExpressionAst]) {
                return $false
            }
            if ($null -ne $scope.Function) { return $true }
            return $null -eq (Get-AmeR2cRAstOwningFunction -Node $node)
        }, $true))
        foreach ($member in $members) {
            $memberName = [string]$member.Member.Value
            if ($memberName -ieq "Delete") {
                throw "R2c-R deletion safety audit rejected path-addressed Delete"
            }
            if ($memberName -iin @("Move", "Replace") -and
                $member.Static -and
                $member.Expression -is [Management.Automation.Language.TypeExpressionAst] -and
                [string]$member.Expression.TypeName.FullName -iin @(
                    "IO.File", "System.IO.File", "IO.Directory", "System.IO.Directory"
                )) {
                $isLockedGuardrailMove = (
                    $memberName -ceq "Move" -and
                    [string]$member.Expression.TypeName.FullName -iin @(
                        "IO.Directory", "System.IO.Directory",
                        "IO.File", "System.IO.File"
                    ) -and
                    -not [string]::IsNullOrEmpty($scope.Record.Path) -and
                    [IO.Path]::GetFileName($scope.Record.Path) -ceq
                        "acceptance_test_r2c_change_driven_reliability_guardrails.ps1" -and
                    $scope.Record.Digest -ceq $script:AmeR2cRGuardrailMoveSourceDigest
                )
                if (-not $isLockedGuardrailMove) {
                    throw "R2c-R deletion safety audit rejected path-addressed move or replace"
                }
            }
            if ($memberName -ieq "MoveTo") {
                throw "R2c-R deletion safety audit rejected path-addressed MoveTo"
            }
            if ($memberName -iin @("Invoke", "InvokeMember", "DynamicInvoke", "CreateDelegate")) {
                throw "R2c-R deletion safety audit rejected reflection invocation"
            }
            if ($member.Static -and $memberName -ieq "Create" -and
                $member.Expression -is [Management.Automation.Language.TypeExpressionAst] -and
                [string]$member.Expression.TypeName.FullName -iin @(
                    "scriptblock", "System.Management.Automation.ScriptBlock"
                )) {
                throw "R2c-R deletion safety audit rejected ScriptBlock.Create"
            }
            if ($memberName -ceq "Start" -and
                $ownerName -cne "Invoke-AmeR2cROwnedProcess") {
                throw "R2c-R deletion safety audit rejected process Start outside the verified boundary"
            }
        }
    }
    Assert-AmeR2cRProcessBoundaryContract -State $State
}

function Assert-AmeR2cRDeletionSafetySource {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][AllowNull()][object]$SourceText,
        [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$Label,
        [AllowNull()][string]$SourcePath,
        [switch]$RetainSnapshots
    )
    if ($null -ne $SourceText -and $SourceText -isnot [string]) {
        throw "R2c-R deletion safety audit source text must be a string"
    }
    $state = New-AmeR2cRDeletionAuditState
    $retain = $false
    try {
        $root = Add-AmeR2cRDeletionAuditSource `
            -State $state `
            -SourceText $SourceText `
            -Label $Label `
            -SourcePath $SourcePath `
            -IsRoot $true `
            -Depth 0
        Invoke-AmeR2cRDeletionAuditClosure -State $state -Root $root
        if ($RetainSnapshots) {
            $retain = $true
            return $state
        }
    } finally {
        if (-not $retain) { Close-AmeR2cRDeletionAuditState -State $state }
    }
}
