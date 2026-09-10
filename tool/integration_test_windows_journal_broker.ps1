$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$cargo = (Get-AmeToolchain).Cargo

Push-Location $repositoryRoot
try {
    $testFilters = @(
        "journal_broker_binary_exposes_protocol_probe_and_requires_scm_for_service_mode",
        "disposable_console_host_preserves_source_and_restarts_with_fresh_identity",
        "console_host_rejects_missing_client_proof_with_bounded_timeout",
        "partial_header_and_body_survive_multiple_host_polls",
        "one_connected_client_does_not_consume_the_second_listener",
        "global_worker_budget_is_shared_and_reclaimed",
        "stop_signal_drains_eight_active_workers_without_detaching",
        "service_remains_start_pending_until_all_listener_handles_are_ready",
        "case_sensitive_containment_keeps_photos_siblings_distinct",
        "caller_pinned_mixed_name_semantics_cannot_be_downgraded_by_backend",
        "pinned_root_revalidation_rejects_rename_and_old_path_replacement",
        "reparse_attributes_are_always_rejected",
        "disclosure_probe_requires_every_ancestor_and_never_opens_content",
        "hung_identity_probe_is_terminated_at_the_absolute_deadline",
        "rename_edges_do_not_leak_external_paths_and_root_self_is_allowed",
        "cancellation_is_connection_scoped_non_sticky_and_bounded",
        "terminal_outbox_applies_backpressure_without_overwriting_entries",
        "broker_absence_and_protocol_mismatch_are_explicit_live_only_states",
        "portable_production_never_invokes_the_journal_factory",
        "installed_production_defers_the_journal_factory_until_after_watcher_start",
        "unrelated_volume_storm_returns_bounded_strictly_advancing_pages",
        "native_page_seam_invokes_one_physical_read_and_returns_partial_proof",
        "durable_pending_old_pairs_after_unrelated_records_and_native_buffer_boundary",
        "carried_same_root_rename_is_one_semantic_candidate",
        "per_root_start_filters_before_containment_and_is_order_independent",
        "semantic_budget_stops_before_whole_rename_and_root_order_cannot_starve",
        "oversized_first_semantic_record_fails_only_its_root_without_livelock",
        "v5_registration_and_capability_round_trip_and_v2_v3_v4_are_rejected",
        "shared_volume_request_round_trips_and_rejects_cross_volume_or_root_replay",
        "shared_volume_read_calls_backend_once_and_isolates_one_root_failure",
        "post_admission_root_failure_keeps_a_healthy_volume_sibling_progressing",
        "shared_volume_read_returns_cross_root_handoff_without_raw_paths",
        "decode_rejects_handoff_bound_to_a_failed_endpoint",
        "shared_client_rejects_outcomes_not_bound_to_original_root_starts",
        "shared_client_rejects_carried_handoff_not_bound_to_original_pending_old",
        "session_reader_uses_one_shared_request_and_preserves_root_failure_isolation",
        "equal_checkpoints_are_successful_no_ops_without_a_shared_read",
        "growing_query_boundary_is_shared_before_any_root_advances_past_it",
        "session_carries_pending_old_into_a_later_cross_root_page",
        "session_rejects_pending_or_carried_rename_outside_durable_request_evidence"
    )
    $executedTests = 0
    foreach ($testFilter in $testFilters) {
        $targetArguments = if (@(
                "journal_broker_binary_exposes_protocol_probe_and_requires_scm_for_service_mode",
                "disposable_console_host_preserves_source_and_restarts_with_fresh_identity",
                "console_host_rejects_missing_client_proof_with_bounded_timeout"
            ) -contains $testFilter) {
            @("--test", "journal_broker_binary")
        } else {
            @("--lib")
        }
        $cargoArguments = @(
            "test",
            "--locked",
            "--manifest-path",
            "rust\Cargo.toml"
        ) + $targetArguments + @(
            "-j",
            "1",
            $testFilter
        )
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = @(& $cargo @cargoArguments 2>&1)
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        if ($exitCode -ne 0) {
            throw "$cargo failed for $testFilter with exit code $exitCode`n" +
                ($output -join "`n")
        }
        $matchingTests = @($output | Where-Object {
            [string]$_ -match '^test .+\.\.\. ok$' -and
            [string]$_ -match [regex]::Escape($testFilter)
        })
        if ($matchingTests.Count -ne 1) {
            throw "Journal broker integration filter $testFilter executed " +
                "$($matchingTests.Count) tests instead of exactly one"
        }
        $executedTests += $matchingTests.Count
        $output | Write-Output
    }
    if ($executedTests -ne $testFilters.Count) {
        throw "Journal broker integration executed an incomplete focused test set"
    }
} finally {
    Pop-Location
}

Write-Output "windows_journal_broker_integration_passed"
