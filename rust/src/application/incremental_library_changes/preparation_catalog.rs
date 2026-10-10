use crate::domain::{
    AssetLocationView, FileIdentityEvidence, LibraryChangeCatchUpEvidence,
    LibraryChangeQueuePolicy, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

// A path preparation performs at most two reads; a paired rename performs at most five.
// Overflow retires only this optional proof, never the underlying reconciliation work.
const MAX_READS: usize = LibraryChangeQueuePolicy::MAX_LEASE_BATCH as usize * 5;
const MAX_RETAINED_BYTES: usize = 8 * 1_024 * 1_024;

pub(super) struct PreparationCatalog<'a, Repository> {
    repository: &'a Repository,
    reads: PreparationReadSet,
}

impl<'a, Repository: IncrementalCatalogRepository> PreparationCatalog<'a, Repository> {
    pub(super) fn new(repository: &'a Repository) -> Self {
        Self {
            repository,
            reads: PreparationReadSet::default(),
        }
    }

    pub(super) fn untracked(repository: &'a Repository) -> Self {
        Self {
            repository,
            reads: PreparationReadSet {
                reads: None,
                retained_bytes: 0,
            },
        }
    }

    pub(super) fn finish(self) -> PreparationReadSet {
        self.reads
    }

    pub(super) fn load_incremental_location_by_relative_path(
        &mut self,
        root_id: &str,
        relative_path: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        let location = self
            .repository
            .load_incremental_location_by_relative_path(root_id, relative_path)?;
        self.reads.record(
            root_id.len().saturating_add(relative_path.len()),
            &location,
            || PreparationQuery::Path {
                root_id: root_id.to_owned(),
                relative_path: relative_path.to_owned(),
            },
        );
        Ok(location)
    }

    pub(super) fn load_incremental_location_by_file_identity(
        &mut self,
        identity: &FileIdentityEvidence,
        lineage: &[LibraryChangeCatchUpEvidence],
    ) -> Result<Option<AssetLocationView>, ScanError> {
        let location = self
            .repository
            .load_incremental_location_by_file_identity(identity, lineage)?;
        let key_bytes = lineage.iter().fold(
            identity.scheme.len().saturating_add(identity.value.len()),
            |bytes, evidence| {
                bytes
                    .saturating_add(std::mem::size_of_val(evidence))
                    .saturating_add(evidence.source.len())
                    .saturating_add(evidence.watermark.len())
            },
        );
        self.reads
            .record(key_bytes, &location, || PreparationQuery::Identity {
                identity: identity.clone(),
                lineage: lineage.to_vec(),
            });
        Ok(location)
    }
}

enum PreparationQuery {
    Path {
        root_id: String,
        relative_path: String,
    },
    Identity {
        identity: FileIdentityEvidence,
        lineage: Vec<LibraryChangeCatchUpEvidence>,
    },
}

struct PreparationRead {
    query: PreparationQuery,
    location: Option<AssetLocationView>,
}

pub(super) struct PreparationReadSet {
    reads: Option<Vec<PreparationRead>>,
    retained_bytes: usize,
}

impl Default for PreparationReadSet {
    fn default() -> Self {
        Self {
            reads: Some(Vec::new()),
            retained_bytes: 0,
        }
    }
}

impl PreparationReadSet {
    fn record(
        &mut self,
        key_bytes: usize,
        location: &Option<AssetLocationView>,
        query: impl FnOnce() -> PreparationQuery,
    ) {
        let Some(reads) = &mut self.reads else { return };
        let bytes = key_bytes
            .saturating_add(std::mem::size_of::<PreparationRead>())
            .saturating_add(location.as_ref().map_or(0, location_bytes));
        let total = self.retained_bytes.saturating_add(bytes);
        if reads.len() >= MAX_READS || total > MAX_RETAINED_BYTES {
            self.reads = None;
            self.retained_bytes = 0;
            return;
        }
        // Preserve every original observation. Inconsistent repeated reads must never collapse
        // into a last-write-wins witness that could certify a mixed preparation.
        reads.push(PreparationRead {
            query: query(),
            location: location.clone(),
        });
        self.retained_bytes = total;
    }

    pub(super) fn still_matches(
        &self,
        repository: &impl IncrementalCatalogRepository,
        cancelled: Option<&std::sync::atomic::AtomicBool>,
    ) -> bool {
        let Some(reads) = &self.reads else {
            return false;
        };
        for read in reads {
            if super::cancellation_requested(cancelled) {
                return false;
            }
            let current = match &read.query {
                PreparationQuery::Path {
                    root_id,
                    relative_path,
                } => repository.load_incremental_location_by_relative_path(root_id, relative_path),
                PreparationQuery::Identity { identity, lineage } => {
                    repository.load_incremental_location_by_file_identity(identity, lineage)
                }
            };
            // A failed proof lookup falls back to ordinary preparation and its existing
            // per-change error policy; it can never authorize reuse.
            if !matches!(current, Ok(ref location) if *location == read.location) {
                return false;
            }
        }
        true
    }
}

fn location_bytes(location: &AssetLocationView) -> usize {
    let AssetLocationView {
        asset_id,
        location_id,
        root_id,
        scan_id,
        absolute_path,
        display_path,
        relative_path,
        preview_path,
        file_size: _,
        created_unix_ms: _,
        modified_unix_ms: _,
        file_identity,
        source_revision,
        source_generation: _,
        width: _,
        height: _,
        preview_status: _,
        preview_issue_code,
        preview_issue_message,
        metadata_engine_id,
        metadata_engine_version,
        capture_time,
    } = location;
    let strings = [
        asset_id.as_str(),
        location_id,
        root_id,
        scan_id,
        absolute_path,
        display_path,
        relative_path,
        preview_path,
        preview_issue_code.as_deref().unwrap_or_default(),
        preview_issue_message.as_deref().unwrap_or_default(),
        metadata_engine_id,
        metadata_engine_version,
    ];
    let mut bytes = strings
        .iter()
        .fold(std::mem::size_of::<AssetLocationView>(), |bytes, value| {
            bytes.saturating_add(value.len())
        });
    if let Some(identity) = file_identity {
        bytes = bytes
            .saturating_add(identity.scheme.len())
            .saturating_add(identity.value.len());
    }
    if let Some(revision) = source_revision {
        bytes = bytes
            .saturating_add(revision.scheme.len())
            .saturating_add(revision.value.len());
    }
    if let Some(capture) = capture_time {
        bytes = bytes
            .saturating_add(capture.local_time.len())
            .saturating_add(capture.raw_value.len());
    }
    bytes
}
