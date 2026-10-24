use super::codec::Cell;

pub(super) enum Field {
    Text,
    Integer,
    OptionalText,
    OptionalInteger,
}

impl Field {
    pub fn accepts(&self, value: &Cell) -> bool {
        matches!(
            (self, value),
            (Self::Text | Self::OptionalText, Cell::Text(_))
                | (Self::Integer | Self::OptionalInteger, Cell::Integer(_))
                | (Self::OptionalText | Self::OptionalInteger, Cell::Null)
        )
    }
}

pub(super) struct RecordKind {
    pub name: &'static str,
    pub fields: &'static [Field],
    pub query: &'static str,
}

use Field::{Integer as I, OptionalInteger as NI, OptionalText as NT, Text as T};

// Field order belongs to the versioned archive contract, never catalog schema discovery.
pub(super) const KINDS: &[RecordKind] = &[
    RecordKind {
        name: "album", fields: &[T, T, I, I, I],
        query: "SELECT album_id, name, revision, created_unix_ms, updated_unix_ms
                FROM organization_albums ORDER BY album_id",
    },
    RecordKind {
        name: "record", fields: &[T, NT, T, I, I, I, I, I, NI],
        query: "SELECT record_id, album_id, disposition, favorite, organization_revision,
                       favorite_revision, created_unix_ms, updated_unix_ms, unassociated_since_unix_ms
                FROM organization_records ORDER BY record_id",
    },
    RecordKind {
        name: "binding", fields: &[T, T, I, I, NT, NT, NT, NT, NT],
        query: "SELECT record_id, asset_id, source_generation, byte_length, file_identity_scheme,
                       file_identity_value, source_revision_token, fingerprint_scheme, digest
                FROM organization_bindings ORDER BY asset_id, source_generation",
    },
    RecordKind {
        name: "source_owner", fields: &[T, T],
        query: "SELECT root_id, record_id FROM organization_sources ORDER BY root_id, record_id",
    },
    RecordKind {
        name: "group", fields: &[T, T],
        query: "SELECT record_id, group_id FROM organization_record_groups ORDER BY record_id",
    },
    RecordKind {
        name: "conflict", fields: &[T, T, I],
        query: "SELECT record_id, album_id, decision_revision
                FROM organization_conflicts ORDER BY record_id, album_id",
    },
    RecordKind {
        name: "operation", fields: &[T, T, T, I, I, I],
        query: "SELECT operation_id, request_digest, operation_kind, revision, changed_groups, created_unix_ms
                FROM organization_operations ORDER BY operation_id",
    },
    RecordKind {
        name: "operation_change", fields: &[T, T, T, NT, NT, NI, I],
        query: "SELECT operation_id, record_id, field, previous_album_id, previous_disposition,
                       previous_favorite, applied_revision
                FROM organization_operation_changes ORDER BY operation_id, record_id, field",
    },
    RecordKind {
        name: "retired_album", fields: &[T, I, I],
        query: "SELECT album_id, revision, retired_unix_ms FROM organization_album_retirements ORDER BY album_id",
    },
    RecordKind {
        name: "album_change", fields: &[T, T, NT, I, I],
        query: "SELECT operation_id, album_id, previous_name, previous_retired, applied_revision
                FROM organization_album_operation_changes ORDER BY operation_id",
    },
    RecordKind {
        name: "operation_conflict", fields: &[T, T, T, I],
        query: "SELECT operation_id, record_id, album_id, decision_revision
                FROM organization_operation_conflicts ORDER BY operation_id, record_id, album_id",
    },
    RecordKind {
        name: "operation_target", fields: &[T, T, T],
        query: "SELECT operation_id, record_id, field FROM organization_operation_targets
                ORDER BY operation_id, record_id, field",
    },
    RecordKind {
        name: "review_scope", fields: &[T, NT, NT, NT, I, T, T],
        query: "SELECT collection, album_id, root_id, folder_relative_path, include_descendants,
                       search_text, queue FROM organization_review_scopes WHERE slot = 'bookmark'",
    },
    RecordKind {
        name: "bookmark", fields: &[T, I, T, T, I, I],
        query: "SELECT record_id, admission_id, root_id, location_id, after_current, updated_unix_ms
                FROM organization_review_bookmark WHERE singleton = 1",
    },
    RecordKind {
        name: "review_operation", fields: &[T, T, I, I],
        query: "SELECT operation_id, session_id, position_sequence, committed_sequence
                FROM organization_review_operations ORDER BY operation_id",
    },
    RecordKind {
        name: "admission", fields: &[I, I],
        query: "SELECT admission_id, source_generation FROM organization_admissions
                UNION ALL SELECT admission_id, source_generation FROM organization_recovery_admissions
                ORDER BY admission_id",
    },
    RecordKind {
        name: "admission_source", fields: &[I, T],
        query: "SELECT source.admission_id, root.root_id FROM organization_admission_sources AS source
                JOIN organization_admission_roots AS root ON root.root_key = source.root_key
                UNION ALL SELECT history.admission_id, source.root_id
                FROM organization_recovery_admissions AS history
                JOIN organization_recovery_admission_sources AS source USING(source_generation)
                ORDER BY admission_id, root_id",
    },
    RecordKind {
        name: "root_evidence", fields: &[T, T, NT, NT],
        query: "SELECT root.id AS root_id, root.path, namespace.identity_scheme, namespace.identity_value
                FROM library_roots AS root
                LEFT JOIN library_change_root_state AS state ON state.root_id = root.id
                LEFT JOIN library_root_publication_namespaces AS namespace
                  ON namespace.root_id = root.id AND namespace.root_generation = state.generation
                UNION ALL
                SELECT retained.root_id, retained.path, retained.identity_scheme, retained.identity_value
                FROM organization_recovery_roots AS retained
                WHERE NOT EXISTS(SELECT 1 FROM library_roots AS root WHERE root.id = retained.root_id)
                ORDER BY root_id",
    },
    RecordKind {
        name: "version_evidence", fields: &[T, T, T, I, I, NT, NT, NT, NT, NT, I],
        query: "WITH current AS MATERIALIZED (
                SELECT l.location_id, l.asset_id, l.root_id, l.source_generation, l.file_size AS byte_length,
                       l.file_identity_scheme, l.file_identity_value, l.source_revision_token,
                       f.scheme AS fingerprint_scheme, f.digest
                FROM asset_locations AS l
                JOIN library_roots AS root ON root.id = l.root_id AND root.active_scan_id = l.scan_id
                JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
                LEFT JOIN content_fingerprints AS f ON f.source_generation = l.source_generation
                  AND f.scheme = 'blake3-256-full-v1' AND f.byte_length = l.file_size
                  AND f.file_identity_scheme IS l.file_identity_scheme
                  AND f.file_identity_value IS l.file_identity_value
                  AND f.source_revision_token IS l.source_revision_token
                ), copies AS MATERIALIZED (
                  SELECT file_identity_scheme, file_identity_value, COUNT(*) AS native_copy_count
                  FROM current WHERE file_identity_scheme = 'windows-file-id-128-v1'
                  GROUP BY file_identity_scheme, file_identity_value
                )
                SELECT current.location_id, current.asset_id, current.root_id, current.source_generation, current.byte_length,
                  current.file_identity_scheme, current.file_identity_value, current.source_revision_token,
                  current.fingerprint_scheme, current.digest, COALESCE(copies.native_copy_count, 1) FROM current
                LEFT JOIN copies USING(file_identity_scheme, file_identity_value)
                UNION ALL SELECT location_id, asset_id, root_id, source_generation, byte_length,
                  file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest,
                  native_copy_count
                FROM organization_recovery_versions
                ORDER BY location_id",
    },
    RecordKind {
        name: "duplicate_decision", fields: &[T, T, I, NT, NT, I, I],
        query: "SELECT scheme, digest, byte_length, preferred_location_id, preferred_asset_id,
                       ignored, updated_unix_ms FROM exact_duplicate_decisions
                UNION ALL SELECT scheme, digest, byte_length, preferred_location_id, preferred_asset_id,
                       ignored, updated_unix_ms FROM organization_recovery_duplicate_decisions AS retained
                WHERE NOT EXISTS(SELECT 1 FROM exact_duplicate_decisions AS current
                  WHERE current.scheme = retained.scheme AND current.digest = retained.digest AND current.byte_length = retained.byte_length)
                ORDER BY scheme, digest, byte_length",
    },
];
