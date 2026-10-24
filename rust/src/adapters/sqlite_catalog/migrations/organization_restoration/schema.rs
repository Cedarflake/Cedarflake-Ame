pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "organization_archive_restores",
        "CREATE TABLE organization_archive_restores (
          archive_id TEXT PRIMARY KEY NOT NULL,
          checksum TEXT NOT NULL CHECK(length(checksum) = 64 AND checksum NOT GLOB '*[^0-9a-f]*'),
          operation_id TEXT NOT NULL UNIQUE REFERENCES organization_operations(operation_id),
          revision_base INTEGER NOT NULL CHECK(revision_base > 0),
          source_revision INTEGER NOT NULL CHECK(source_revision >= 0),
          restored_unix_ms INTEGER NOT NULL CHECK(restored_unix_ms >= 0)
        )",
    ),
    (
        "table",
        "organization_restored_fields",
        "CREATE TABLE organization_restored_fields (
          record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
          field TEXT NOT NULL CHECK(field IN ('organization', 'favorite')),
          applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
          PRIMARY KEY(record_id, field)
        ) WITHOUT ROWID",
    ),
    (
        "table",
        "organization_archive_generations",
        "CREATE TABLE organization_archive_generations (
          archive_id TEXT NOT NULL REFERENCES organization_archive_restores(archive_id),
          source_generation INTEGER NOT NULL CHECK(source_generation > 0),
          local_generation INTEGER NOT NULL UNIQUE CHECK(local_generation > 0),
          PRIMARY KEY(archive_id, source_generation)
        ) WITHOUT ROWID",
    ),
    (
        "table",
        "organization_recovery_roots",
        "CREATE TABLE organization_recovery_roots (
          root_id TEXT PRIMARY KEY NOT NULL,
          archive_id TEXT NOT NULL REFERENCES organization_archive_restores(archive_id),
          original_root_id TEXT NOT NULL, path TEXT NOT NULL,
          identity_scheme TEXT, identity_value TEXT,
          CHECK((identity_scheme IS NULL) = (identity_value IS NULL)),
          UNIQUE(archive_id, original_root_id)
        )",
    ),
    (
        "table",
        "organization_recovery_versions",
        "CREATE TABLE organization_recovery_versions (
          location_id TEXT PRIMARY KEY NOT NULL, asset_id TEXT NOT NULL, root_id TEXT NOT NULL,
          source_generation INTEGER NOT NULL CHECK(source_generation > 0),
          byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
          file_identity_scheme TEXT, file_identity_value TEXT, source_revision_token TEXT,
          fingerprint_scheme TEXT, digest TEXT,
          native_copy_count INTEGER NOT NULL CHECK(native_copy_count > 0),
          CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
          CHECK((fingerprint_scheme IS NULL) = (digest IS NULL)),
          CHECK(digest IS NULL OR (length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'))
        )",
    ),
    (
        "index",
        "organization_recovery_versions_native",
        "CREATE INDEX organization_recovery_versions_native ON organization_recovery_versions(
          file_identity_scheme, file_identity_value, source_revision_token, byte_length)",
    ),
    (
        "index",
        "organization_recovery_versions_content",
        "CREATE INDEX organization_recovery_versions_content ON organization_recovery_versions(
          fingerprint_scheme, digest, byte_length)",
    ),
    (
        "index",
        "organization_recovery_versions_generation",
        "CREATE INDEX organization_recovery_versions_generation ON organization_recovery_versions(source_generation)",
    ),
    (
        "table",
        "organization_recovery_admissions",
        "CREATE TABLE organization_recovery_admissions (
          source_generation INTEGER PRIMARY KEY CHECK(source_generation > 0),
          admission_id INTEGER NOT NULL UNIQUE CHECK(admission_id > 0)
        )",
    ),
    (
        "table",
        "organization_recovery_admission_sources",
        "CREATE TABLE organization_recovery_admission_sources (
          source_generation INTEGER NOT NULL REFERENCES organization_recovery_admissions(source_generation) ON DELETE CASCADE,
          root_id TEXT NOT NULL, PRIMARY KEY(source_generation, root_id)
        ) WITHOUT ROWID",
    ),
    (
        "index",
        "organization_recovery_admission_sources_root",
        "CREATE INDEX organization_recovery_admission_sources_root ON organization_recovery_admission_sources(root_id, source_generation)",
    ),
    (
        "table",
        "organization_recovery_admission_links",
        "CREATE TABLE organization_recovery_admission_links (
          source_generation INTEGER PRIMARY KEY REFERENCES organization_admissions(source_generation) ON DELETE CASCADE,
          recovered_generation INTEGER NOT NULL REFERENCES organization_recovery_admissions(source_generation) ON DELETE CASCADE
        )",
    ),
    (
        "view",
        "organization_queue_versions",
        "CREATE VIEW organization_queue_versions AS
          SELECT CASE WHEN history.admission_id < current.admission_id
            THEN history.admission_id ELSE current.admission_id END AS admission_id,
            current.source_generation
          FROM organization_admissions AS current
          LEFT JOIN organization_recovery_admission_links AS linked
            ON linked.source_generation = current.source_generation
          LEFT JOIN organization_recovery_admissions AS history
            ON history.source_generation = linked.recovered_generation",
    ),
    (
        "table",
        "organization_recovery_duplicate_decisions",
        "CREATE TABLE organization_recovery_duplicate_decisions (
          scheme TEXT NOT NULL, digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
          byte_length INTEGER NOT NULL CHECK(byte_length >= 0), preferred_location_id TEXT, preferred_asset_id TEXT,
          ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)), updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          unassociated_since_unix_ms INTEGER NOT NULL CHECK(unassociated_since_unix_ms >= 0),
          CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)), PRIMARY KEY(scheme, digest, byte_length)
        ) WITHOUT ROWID",
    ),
    (
        "table",
        "organization_recovery_duplicate_sources",
        "CREATE TABLE organization_recovery_duplicate_sources (
          scheme TEXT NOT NULL, digest TEXT NOT NULL, byte_length INTEGER NOT NULL, root_id TEXT NOT NULL,
          FOREIGN KEY(scheme, digest, byte_length) REFERENCES organization_recovery_duplicate_decisions(scheme, digest, byte_length) ON DELETE CASCADE,
          PRIMARY KEY(scheme, digest, byte_length, root_id)
        ) WITHOUT ROWID",
    ),
];
