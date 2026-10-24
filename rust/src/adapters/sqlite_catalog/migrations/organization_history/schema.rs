pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "organization_history_state", "CREATE TABLE organization_history_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        cursor INTEGER NOT NULL CHECK(cursor >= 0),
        started_revision INTEGER NOT NULL CHECK(started_revision >= 0)
    )"),
    ("table", "organization_history_entries", "CREATE TABLE organization_history_entries (
        position INTEGER PRIMARY KEY CHECK(position > 0),
        operation_id TEXT UNIQUE NOT NULL,
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE
    )"),
    ("table", "organization_field_authority", "CREATE TABLE organization_field_authority (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        previous_authority INTEGER NOT NULL CHECK(previous_authority >= 0 AND previous_authority < applied_revision),
        applied_authority INTEGER NOT NULL CHECK(applied_authority >= 0 AND applied_authority <= applied_revision),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id, record_id, field)
            REFERENCES organization_operation_changes(operation_id, record_id, field) ON DELETE CASCADE
    )"),
    ("index", "organization_field_authority_revision", "CREATE UNIQUE INDEX organization_field_authority_revision
        ON organization_field_authority(record_id, field, applied_revision)"),
    ("table", "organization_album_authority", "CREATE TABLE organization_album_authority (
        operation_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT NOT NULL,
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        previous_authority INTEGER NOT NULL CHECK(previous_authority >= 0 AND previous_authority < applied_revision),
        applied_authority INTEGER NOT NULL CHECK(applied_authority >= 0 AND applied_authority <= applied_revision),
        FOREIGN KEY(operation_id) REFERENCES organization_album_operation_changes(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    )"),
    ("index", "organization_album_authority_revision", "CREATE UNIQUE INDEX organization_album_authority_revision
        ON organization_album_authority(album_id, applied_revision)"),
    ("trigger", "organization_capture_field_authority", "CREATE TRIGGER organization_capture_field_authority
        AFTER INSERT ON organization_operation_changes
        WHEN NEW.applied_revision = (SELECT revision + 1 FROM organization_state) BEGIN
        INSERT INTO organization_field_authority
            (operation_id, record_id, field, applied_revision, previous_authority, applied_authority)
        SELECT NEW.operation_id, NEW.record_id, NEW.field, NEW.applied_revision,
            COALESCE(prior.applied_authority,
                CASE NEW.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END),
            NEW.applied_revision
        FROM organization_records AS record
        LEFT JOIN organization_field_authority AS prior ON prior.record_id = record.record_id
            AND prior.field = NEW.field AND prior.applied_revision =
                CASE NEW.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END
        WHERE record.record_id = NEW.record_id;
    END"),
    ("trigger", "organization_capture_album_authority", "CREATE TRIGGER organization_capture_album_authority
        AFTER INSERT ON organization_album_operation_changes
        WHEN NEW.applied_revision = (SELECT revision + 1 FROM organization_state) BEGIN
        INSERT INTO organization_album_authority
            (operation_id, album_id, applied_revision, previous_authority, applied_authority)
        SELECT NEW.operation_id, NEW.album_id, NEW.applied_revision,
            CASE WHEN NEW.previous_name IS NULL THEN 0 ELSE COALESCE(prior.applied_authority, album.revision) END,
            NEW.applied_revision
        FROM organization_albums AS album
        LEFT JOIN organization_album_authority AS prior ON prior.album_id = album.album_id
            AND prior.applied_revision = album.revision
        WHERE album.album_id = NEW.album_id;
    END"),
];
