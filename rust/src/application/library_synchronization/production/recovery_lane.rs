use std::collections::BTreeMap;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    LibraryChangeQueuePolicy, LibraryRootGeneration, LibrarySynchronizationSnapshot, ScanError,
};

const RECOVERY_RETRY_INITIAL_MILLIS: i64 = 1_000;
const RECOVERY_RETRY_MAXIMUM_MILLIS: i64 = 5 * 60 * 1_000;

#[derive(Default)]
pub(super) struct RecoveryLane {
    root_cursor: Option<String>,
    retries: BTreeMap<String, RecoveryRetryState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecoveryRetryState {
    failure_count: u32,
    next_attempt_unix_ms: i64,
}

pub(super) enum ReadyRecoveryWork {
    CandidateDrain {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
    LegacyUnownedDrain {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
    Control {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
}

impl ReadyRecoveryWork {
    pub(super) fn root_id(&self) -> &str {
        match self {
            Self::CandidateDrain { root_id, .. }
            | Self::LegacyUnownedDrain { root_id, .. }
            | Self::Control { root_id, .. } => root_id,
        }
    }

    pub(super) const fn root_generation(&self) -> LibraryRootGeneration {
        match self {
            Self::CandidateDrain {
                root_generation, ..
            }
            | Self::LegacyUnownedDrain {
                root_generation, ..
            }
            | Self::Control {
                root_generation, ..
            } => *root_generation,
        }
    }
}

impl RecoveryLane {
    pub(super) fn select(
        &mut self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
        journal_capacity_debt_roots: &[(String, LibraryRootGeneration)],
    ) -> Result<Option<ReadyRecoveryWork>, ScanError> {
        if snapshot.roots.is_empty() {
            return Ok(None);
        }
        let start_index = self
            .root_cursor
            .as_ref()
            .and_then(|root_id| {
                snapshot
                    .roots
                    .iter()
                    .position(|root| &root.root_id == root_id)
            })
            .map_or(0, |index| (index + 1) % snapshot.roots.len());
        for offset in 0..snapshot.roots.len() {
            let root = &snapshot.roots[(start_index + offset) % snapshot.roots.len()];
            if root.availability != crate::domain::LibraryRootAvailability::Available
                || root.source_health != crate::domain::LibraryChangeSourceHealth::Healthy
                || root.recovery_blocked
                || !self.is_due(&root.root_id, now_unix_ms)
            {
                continue;
            }
            let root_generation = crate::domain::LibraryRootGeneration::new(root.root_generation)
                .ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
            if !journal_capacity_debt_roots.is_empty()
                && !journal_capacity_debt_roots
                    .iter()
                    .any(|(root_id, generation)| {
                        root_id == &root.root_id && *generation == root_generation
                    })
            {
                continue;
            }
            if catalog.has_ready_metadata_inventory_recovery_candidates(
                &root.root_id,
                root_generation,
                now_unix_ms,
                policy,
            )? {
                self.root_cursor = Some(root.root_id.clone());
                return Ok(Some(ReadyRecoveryWork::CandidateDrain {
                    root_id: root.root_id.clone(),
                    root_generation,
                }));
            }
            if catalog.has_ready_legacy_unowned_recovery_debt(
                &root.root_id,
                root_generation,
                now_unix_ms,
                policy,
            )? {
                self.root_cursor = Some(root.root_id.clone());
                return Ok(Some(ReadyRecoveryWork::LegacyUnownedDrain {
                    root_id: root.root_id.clone(),
                    root_generation,
                }));
            }
            if !journal_capacity_debt_roots.is_empty() {
                continue;
            }
            if catalog.has_unresolved_metadata_inventory_recovery_candidates(
                &root.root_id,
                root_generation,
            )? {
                continue;
            }
            if catalog.has_ready_metadata_inventory_recovery(
                &root.root_id,
                root_generation,
                now_unix_ms,
                policy,
            )? {
                self.root_cursor = Some(root.root_id.clone());
                return Ok(Some(ReadyRecoveryWork::Control {
                    root_id: root.root_id.clone(),
                    root_generation,
                }));
            }
        }
        Ok(None)
    }

    fn is_due(&self, root_id: &str, now_unix_ms: i64) -> bool {
        self.retries
            .get(root_id)
            .is_none_or(|retry| now_unix_ms >= retry.next_attempt_unix_ms)
    }

    pub(super) fn record_failure(&mut self, root_id: &str, now_unix_ms: i64) {
        let failure_count = self
            .retries
            .get(root_id)
            .map_or(1, |retry| retry.failure_count.saturating_add(1));
        let exponent = failure_count.saturating_sub(1).min(31);
        let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
        let delay = RECOVERY_RETRY_INITIAL_MILLIS
            .saturating_mul(multiplier)
            .min(RECOVERY_RETRY_MAXIMUM_MILLIS);
        self.retries.insert(
            root_id.to_owned(),
            RecoveryRetryState {
                failure_count,
                next_attempt_unix_ms: now_unix_ms.saturating_add(delay),
            },
        );
    }

    pub(super) fn record_success(&mut self, root_id: &str) {
        self.retries.remove(root_id);
    }

    #[cfg(test)]
    pub(super) fn retry_for_test(&self, root_id: &str) -> Option<(u32, i64)> {
        self.retries
            .get(root_id)
            .map(|retry| (retry.failure_count, retry.next_attempt_unix_ms))
    }
}

#[cfg(test)]
mod tests;
