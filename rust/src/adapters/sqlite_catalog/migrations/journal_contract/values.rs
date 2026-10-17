use crate::domain::{
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeScope, LibraryRootGeneration,
    PersistentJournalCapabilityState, PersistentJournalContinuityState, PersistentJournalFailure,
    PersistentJournalLineageState, PersistentJournalRangeState, ScanError,
};

pub(in super::super) fn parse_root_generation(
    value: i64,
) -> Result<LibraryRootGeneration, ScanError> {
    u64::try_from(value)
        .ok()
        .and_then(LibraryRootGeneration::new)
        .ok_or_else(unverifiable_contract)
}

pub(in super::super) fn parse_canonical_u64(value: &str) -> Result<u64, ScanError> {
    let parsed = value.parse::<u64>().map_err(|_| unverifiable_contract())?;
    if parsed.to_string() != value {
        return Err(unverifiable_contract());
    }
    Ok(parsed)
}

pub(in super::super) fn parse_capability_state(
    value: &str,
) -> Result<PersistentJournalCapabilityState, ScanError> {
    match value {
        "unknown" => Ok(PersistentJournalCapabilityState::Unknown),
        "supported" => Ok(PersistentJournalCapabilityState::Supported),
        "live_only" => Ok(PersistentJournalCapabilityState::LiveOnly),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_continuity_state(
    value: &str,
) -> Result<PersistentJournalContinuityState, ScanError> {
    match value {
        "baseline_required" => Ok(PersistentJournalContinuityState::BaselineRequired),
        "catching_up" => Ok(PersistentJournalContinuityState::CatchingUp),
        "current" => Ok(PersistentJournalContinuityState::Current),
        "recovery_required" => Ok(PersistentJournalContinuityState::RecoveryRequired),
        "live_only" => Ok(PersistentJournalContinuityState::LiveOnly),
        "unavailable" => Ok(PersistentJournalContinuityState::Unavailable),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_range_state(
    value: &str,
) -> Result<PersistentJournalRangeState, ScanError> {
    match value {
        "enrolled" => Ok(PersistentJournalRangeState::Enrolled),
        "checkpointed" => Ok(PersistentJournalRangeState::Checkpointed),
        "superseded" => Ok(PersistentJournalRangeState::Superseded),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_lineage_state(
    value: &str,
) -> Result<PersistentJournalLineageState, ScanError> {
    match value {
        "pending" => Ok(PersistentJournalLineageState::Pending),
        "completed" => Ok(PersistentJournalLineageState::Completed),
        "superseded" => Ok(PersistentJournalLineageState::Superseded),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_failure(
    code: Option<String>,
    message: Option<String>,
) -> Result<Option<PersistentJournalFailure>, ScanError> {
    match (code, message) {
        (None, None) => Ok(None),
        (Some(code), Some(message)) => Ok(Some(PersistentJournalFailure { code, message })),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn unverifiable_contract() -> ScanError {
    ScanError::new(
        "catalog_persistent_journal_contract_unverifiable",
        "The catalog cannot prove its per-root persistent journal continuity authority",
    )
}

pub(in super::super) fn parse_intent_kind(
    value: &str,
) -> Result<LibraryChangeIntentKind, ScanError> {
    match value {
        "reconcile" => Ok(LibraryChangeIntentKind::Reconcile),
        "rename_candidate" => Ok(LibraryChangeIntentKind::RenameCandidate),
        "freshness_unknown" => Ok(LibraryChangeIntentKind::FreshnessUnknown),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_intent_scope(value: &str) -> Result<LibraryChangeScope, ScanError> {
    match value {
        "path" => Ok(LibraryChangeScope::Path),
        "subtree" => Ok(LibraryChangeScope::Subtree),
        "root" => Ok(LibraryChangeScope::Root),
        _ => Err(unverifiable_contract()),
    }
}

pub(in super::super) fn parse_intent_origin(value: &str) -> Result<LibraryChangeOrigin, ScanError> {
    match value {
        "live_notification" => Ok(LibraryChangeOrigin::LiveNotification),
        "metadata_inventory" => Ok(LibraryChangeOrigin::MetadataInventory),
        "startup_catch_up" => Ok(LibraryChangeOrigin::StartupCatchUp),
        "user_refresh" => Ok(LibraryChangeOrigin::UserRefresh),
        "consistency_audit" => Ok(LibraryChangeOrigin::ConsistencyAudit),
        _ => Err(unverifiable_contract()),
    }
}
