pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "organization_state", "CREATE TABLE organization_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0)
    )"),
    ("table", "organization_albums", "CREATE TABLE organization_albums (
        album_id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
        revision INTEGER NOT NULL CHECK(revision > 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
    )"),
    ("table", "organization_records", "CREATE TABLE organization_records (
        record_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT,
        disposition TEXT NOT NULL CHECK(disposition IN ('pending', 'deferred', 'organized', 'ignored')),
        favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
        organization_revision INTEGER NOT NULL CHECK(organization_revision >= 0),
        favorite_revision INTEGER NOT NULL CHECK(favorite_revision >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
        unassociated_since_unix_ms INTEGER CHECK(unassociated_since_unix_ms >= 0),
        CHECK((disposition = 'organized') = (album_id IS NOT NULL)),
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("table", "organization_bindings", "CREATE TABLE organization_bindings (
        record_id TEXT NOT NULL,
        asset_id TEXT NOT NULL,
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
        file_identity_scheme TEXT,
        file_identity_value TEXT,
        source_revision_token TEXT,
        fingerprint_scheme TEXT,
        digest TEXT CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
        CHECK((fingerprint_scheme IS NULL) = (digest IS NULL)),
        PRIMARY KEY(asset_id, source_generation),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    )"),
    ("index", "organization_bindings_record", "CREATE INDEX organization_bindings_record
        ON organization_bindings(record_id)"),
    ("index", "organization_bindings_content", "CREATE INDEX organization_bindings_content
        ON organization_bindings(fingerprint_scheme, digest, byte_length) WHERE digest IS NOT NULL"),
    ("table", "organization_sources", "CREATE TABLE organization_sources (
        root_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        PRIMARY KEY(root_id, record_id),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    )"),
    ("index", "organization_sources_record", "CREATE INDEX organization_sources_record
        ON organization_sources(record_id, root_id)"),
    ("table", "organization_conflicts", "CREATE TABLE organization_conflicts (
        record_id TEXT NOT NULL,
        album_id TEXT NOT NULL,
        decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0),
        PRIMARY KEY(record_id, album_id),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("table", "organization_operations", "CREATE TABLE organization_operations (
        operation_id TEXT PRIMARY KEY NOT NULL,
        request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
        operation_kind TEXT NOT NULL CHECK(operation_kind IN ('favorite', 'organization', 'album', 'undo', 'restore', 'cleanup')),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0)
    )"),
    ("table", "organization_operation_changes", "CREATE TABLE organization_operation_changes (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        previous_album_id TEXT,
        previous_disposition TEXT CHECK(previous_disposition IN ('pending', 'deferred', 'organized', 'ignored')),
        previous_favorite INTEGER CHECK(previous_favorite IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        CHECK((field = 'favorite' AND previous_favorite IS NOT NULL AND previous_disposition IS NULL
                 AND previous_album_id IS NULL)
           OR (field = 'organization' AND previous_favorite IS NULL AND previous_disposition IS NOT NULL
                 AND (previous_disposition = 'organized') = (previous_album_id IS NOT NULL))),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    )"),
];
