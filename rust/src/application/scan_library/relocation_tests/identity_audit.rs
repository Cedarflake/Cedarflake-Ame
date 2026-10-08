use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags, params};

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::{MetadataInventoryEntryKind, MetadataInventoryPlaceholderState};

use super::*;

const MAX_ENTRIES: usize = 120_000;
const MAX_DIRECTORIES: usize = 10_000;
const DEADLINE: Duration = Duration::from_secs(120);

type IdentityKey = (String, String);

struct PreviousLocation {
    root: usize,
    size: Option<u64>,
    modified: i64,
    revision: Option<String>,
}

#[derive(Default)]
struct ObservedIdentity {
    locations: usize,
    metadata_matches: bool,
}

#[derive(Debug, Default)]
struct RootEvidence {
    total: usize,
    missing_identity: usize,
    unique_identity_observed: usize,
    metadata_unchanged: usize,
    metadata_changed_or_unknown: usize,
    ambiguous_identity_observed: usize,
    identity_not_observed: usize,
}

#[derive(Debug, Default)]
struct AuditEvidence {
    roots: [RootEvidence; 2],
    entries: usize,
    directories: usize,
    files: usize,
    unknown_file_identity: usize,
    unrecognized_file_identity: usize,
    skipped_entries: usize,
    complete: bool,
}

fn audit(
    catalog_path: &Path,
    root_ids: [&str; 2],
    destination: &Path,
    limit: usize,
) -> Result<AuditEvidence, String> {
    if root_ids[0] == root_ids[1] || limit == 0 || limit > MAX_ENTRIES {
        return Err("invalid audit scope".into());
    }
    let started = Instant::now();
    let connection = Connection::open_with_flags(catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "retained catalog unavailable")?;
    connection
        .execute_batch("PRAGMA query_only=ON; BEGIN")
        .map_err(|_| "read snapshot unavailable")?;
    connection
        .progress_handler(1000, Some(move || started.elapsed() >= DEADLINE))
        .map_err(|_| "read deadline unavailable")?;
    let mut evidence = AuditEvidence::default();
    let previous = load_previous(&connection, root_ids, &mut evidence)?;
    let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &destination.to_string_lossy(),
        None,
    )
    .map_err(|error| error.code)?;
    let root_identity = guard
        .metadata_inventory_root_identity()
        .map_err(|error| error.code)?
        .ok_or("destination identity unavailable")?;
    let mut pending = VecDeque::from([String::new()]);
    let mut observed: BTreeMap<IdentityKey, ObservedIdentity> = BTreeMap::new();
    'directories: while let Some(relative) = pending.pop_front() {
        if started.elapsed() >= DEADLINE || evidence.directories >= MAX_DIRECTORIES {
            return Err("audit traversal limit exceeded".into());
        }
        let mut entries = guard
            .streaming_metadata_inventory_entries_in_directory(&relative)
            .map_err(|issue| issue.code)?;
        evidence.directories += 1;
        for item in &mut entries {
            if evidence.entries == limit {
                break 'directories;
            }
            if started.elapsed() >= DEADLINE {
                return Err("audit deadline exceeded".into());
            }
            let item = item.map_err(|issue| issue.code)?;
            evidence.entries += 1;
            match item.kind {
                MetadataInventoryEntryKind::Directory => {
                    if item.is_reparse_point
                        || item.placeholder_state != MetadataInventoryPlaceholderState::Available
                    {
                        evidence.skipped_entries += 1;
                    } else {
                        if evidence.directories + pending.len() >= MAX_DIRECTORIES {
                            return Err("audit directory roster limit exceeded".into());
                        }
                        pending.push_back(item.relative_path);
                    }
                }
                MetadataInventoryEntryKind::File => {
                    evidence.files += 1;
                    let Some(identity) = item.file_identity else {
                        evidence.unknown_file_identity += 1;
                        continue;
                    };
                    let key = (identity.scheme, identity.value);
                    let Some(locations) = previous.get(&key) else {
                        evidence.unrecognized_file_identity += 1;
                        continue;
                    };
                    let observation = observed.entry(key).or_default();
                    observation.locations += 1;
                    let old = &locations[0];
                    let revision = item
                        .source_revision
                        .as_ref()
                        .map(|value| format!("{}:{}", value.scheme, value.value));
                    observation.metadata_matches = old.revision.is_some()
                        && old.revision == revision
                        && old.size == item.file_size
                        && old.modified == item.modified_unix_ms;
                }
                MetadataInventoryEntryKind::Other => evidence.skipped_entries += 1,
            }
        }
        entries.finish().map_err(|issue| issue.code)?;
        if pending.is_empty() {
            evidence.complete = evidence.skipped_entries == 0;
        }
    }
    guard
        .require_metadata_inventory_root_identity(&root_identity)
        .map_err(|error| error.code)?;
    for (identity, locations) in &previous {
        let Some(observation) = observed.get(identity) else {
            continue;
        };
        for old in locations {
            let root = &mut evidence.roots[old.root];
            if locations.len() != 1 || observation.locations != 1 {
                root.ambiguous_identity_observed += 1;
                continue;
            }
            root.unique_identity_observed += 1;
            if observation.metadata_matches {
                root.metadata_unchanged += 1;
            } else {
                root.metadata_changed_or_unknown += 1;
            }
        }
    }
    for root in &mut evidence.roots {
        root.identity_not_observed = root.total
            - root.unique_identity_observed
            - root.ambiguous_identity_observed
            - root.missing_identity;
    }
    connection
        .execute_batch("ROLLBACK")
        .map_err(|_| "read snapshot retirement failed")?;
    Ok(evidence)
}

fn load_previous(
    connection: &Connection,
    root_ids: [&str; 2],
    evidence: &mut AuditEvidence,
) -> Result<BTreeMap<IdentityKey, Vec<PreviousLocation>>, String> {
    let mut previous: BTreeMap<IdentityKey, Vec<PreviousLocation>> = BTreeMap::new();
    for (index, root_id) in root_ids.iter().enumerate() {
        let is_published: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id=?1 AND active_scan_id IS NOT NULL)",
            [root_id], |row| row.get(0),
        ).map_err(|_| "retained root unavailable")?;
        if !is_published {
            return Err("retained root has no publication".into());
        }
        let mut statement = connection
            .prepare(
                "SELECT locations.file_identity_scheme, locations.file_identity_value,
                    locations.file_size, locations.modified_unix_ms, locations.source_revision_token
             FROM asset_locations AS locations
             JOIN library_roots AS roots ON roots.id=locations.root_id
                                       AND roots.active_scan_id=locations.scan_id
             WHERE roots.id=?1 LIMIT ?2",
            )
            .map_err(|_| "retained roster query failed")?;
        let rows = statement
            .query_map(params![root_id, 120_001_i64], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|_| "retained roster read failed")?;
        for row in rows {
            let (scheme, value, size, modified, revision) =
                row.map_err(|_| "invalid retained identity row")?;
            let size = size
                .map(u64::try_from)
                .transpose()
                .map_err(|_| "invalid retained file size")?;
            evidence.roots[index].total += 1;
            if evidence.roots.iter().map(|root| root.total).sum::<usize>() > MAX_ENTRIES {
                return Err("retained roster limit exceeded".into());
            }
            match (scheme, value) {
                (Some(scheme), Some(value)) => {
                    previous
                        .entry((scheme, value))
                        .or_default()
                        .push(PreviousLocation {
                            root: index,
                            size,
                            modified,
                            revision,
                        })
                }
                _ => evidence.roots[index].missing_identity += 1,
            }
        }
    }
    Ok(previous)
}

#[test]
#[ignore = "requires current explicit authorization for metadata-only real-source enumeration"]
fn authorized_combined_source_identity_audit() {
    assert_eq!(
        std::env::var("AME_IDENTITY_AUDIT_CONSENT").unwrap(),
        "AME_METADATA_ONLY_IDENTITY_AUDIT_V1"
    );
    let catalog = PathBuf::from(std::env::var("AME_IDENTITY_AUDIT_CATALOG").unwrap());
    let destination = PathBuf::from(std::env::var("AME_IDENTITY_AUDIT_DESTINATION").unwrap());
    let first = std::env::var("AME_IDENTITY_AUDIT_LOCAL_ROOT").unwrap();
    let second = std::env::var("AME_IDENTITY_AUDIT_CLOUD_ROOT").unwrap();
    let canonical_catalog = catalog.canonicalize().unwrap();
    let canonical_destination = destination.canonicalize().unwrap();
    let output_directory = canonical_catalog.parent().unwrap();
    assert!(!canonical_destination.starts_with(output_directory));
    assert!(!output_directory.starts_with(&canonical_destination));
    let result = audit(&catalog, [&first, &second], &destination, MAX_ENTRIES).unwrap();
    let report_path = canonical_catalog.with_extension("identity-audit.txt");
    let mut report = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report_path)
        .unwrap();
    use std::io::Write;
    writeln!(report, "AME_IDENTITY_AUDIT {result:?}").unwrap();
    report.sync_all().unwrap();
    println!("AME_IDENTITY_AUDIT {result:?}");
    assert!(
        result.complete,
        "partial traversal is not complete identity evidence"
    );
}

mod tests;
