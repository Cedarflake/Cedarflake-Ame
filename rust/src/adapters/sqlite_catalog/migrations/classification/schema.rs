pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "classification_categories", "CREATE TABLE classification_categories (
        category_id TEXT PRIMARY KEY NOT NULL CHECK(length(category_id) BETWEEN 1 AND 64),
        label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 80),
        revision INTEGER NOT NULL CHECK(revision >= 0)
    )"),
    ("table", "classification_category_versions", "CREATE TABLE classification_category_versions (
        category_id TEXT NOT NULL REFERENCES classification_categories(category_id),
        semantic_revision TEXT NOT NULL CHECK(length(semantic_revision) = 64),
        descriptions TEXT NOT NULL CHECK(json_valid(descriptions) AND json_type(descriptions) = 'array'
            AND json_array_length(descriptions) BETWEEN 1 AND 8 AND length(CAST(descriptions AS BLOB)) <= 65536),
        PRIMARY KEY(category_id, semantic_revision)
    ) WITHOUT ROWID"),
    ("table", "classification_rules", "CREATE TABLE classification_rules (
        rules_revision TEXT PRIMARY KEY NOT NULL CHECK(length(rules_revision) = 64)
    )"),
    ("table", "classification_rule_members", "CREATE TABLE classification_rule_members (
        rules_revision TEXT NOT NULL REFERENCES classification_rules(rules_revision),
        category_id TEXT NOT NULL,
        semantic_revision TEXT NOT NULL,
        PRIMARY KEY(rules_revision, category_id),
        FOREIGN KEY(category_id, semantic_revision)
            REFERENCES classification_category_versions(category_id, semantic_revision)
    ) WITHOUT ROWID"),
    ("table", "classification_runs", "CREATE TABLE classification_runs (
        run_id TEXT PRIMARY KEY NOT NULL CHECK(length(run_id) BETWEEN 1 AND 128),
        primary_rules TEXT NOT NULL REFERENCES classification_rules(rules_revision),
        configuration_revision TEXT NOT NULL CHECK(length(configuration_revision) = 64),
        payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB)) <= 4096),
        UNIQUE(run_id, configuration_revision)
    )"),
    ("table", "classification_state", "CREATE TABLE classification_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        active_rules TEXT REFERENCES classification_rules(rules_revision),
        configuration_revision TEXT CHECK(configuration_revision IS NULL OR length(configuration_revision) = 64),
        active_run_id TEXT,
        CHECK(active_run_id IS NULL OR configuration_revision IS NOT NULL),
        FOREIGN KEY(active_run_id, configuration_revision) REFERENCES classification_runs(run_id, configuration_revision)
    )"),
    ("table", "classification_predictions", "CREATE TABLE classification_predictions (
        publication_id INTEGER PRIMARY KEY AUTOINCREMENT,
        prediction_id TEXT UNIQUE NOT NULL CHECK(length(prediction_id) BETWEEN 1 AND 128),
        run_id TEXT NOT NULL,
        configuration_revision TEXT NOT NULL,
        fingerprint_scheme TEXT NOT NULL,
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
        payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB)) <= 131072),
        FOREIGN KEY(run_id, configuration_revision) REFERENCES classification_runs(run_id, configuration_revision)
    )"),
    ("index", "classification_predictions_content", "CREATE INDEX classification_predictions_content
        ON classification_predictions(fingerprint_scheme, digest, byte_length, configuration_revision, publication_id DESC)"),
    ("table", "classification_decisions", "CREATE TABLE classification_decisions (
        record_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        category_id TEXT REFERENCES classification_categories(category_id),
        choice TEXT CHECK(choice IS NULL OR (json_valid(choice) AND length(CAST(choice AS BLOB)) <= 4096)),
        choice_revision INTEGER NOT NULL CHECK(choice_revision >= 0),
        choice_restored INTEGER NOT NULL CHECK(choice_restored IN (0, 1)),
        deferred INTEGER NOT NULL CHECK(deferred IN (0, 1)),
        review_revision INTEGER NOT NULL CHECK(review_revision >= 0),
        review_restored INTEGER NOT NULL CHECK(review_restored IN (0, 1)),
        CHECK((category_id IS NULL) = (choice IS NULL)),
        CHECK(choice IS NULL OR choice_revision > 0),
        CHECK(deferred = 0 OR review_revision > 0)
    )"),
    ("index", "classification_decisions_category", "CREATE INDEX classification_decisions_category
        ON classification_decisions(category_id, record_id) WHERE category_id IS NOT NULL"),
    ("table", "classification_operations", "CREATE TABLE classification_operations (
        operation_id TEXT PRIMARY KEY NOT NULL CHECK(length(operation_id) BETWEEN 1 AND 128),
        request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
        operation_kind TEXT NOT NULL CHECK(operation_kind IN ('configure', 'category', 'defer', 'undo', 'restore', 'continuation')),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        restored INTEGER NOT NULL DEFAULT 0 CHECK(restored IN (0, 1))
    )"),
    ("table", "classification_operation_changes", "CREATE TABLE classification_operation_changes (
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        field TEXT NOT NULL CHECK(field IN ('category', 'review')),
        previous_payload TEXT CHECK(previous_payload IS NULL OR (json_valid(previous_payload)
            AND length(CAST(previous_payload AS BLOB)) <= 4096)),
        previous_revision INTEGER NOT NULL CHECK(previous_revision >= 0),
        previous_restored INTEGER NOT NULL CHECK(previous_restored IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        PRIMARY KEY(operation_id, record_id, field),
        CHECK(field <> 'review' OR (previous_payload IS NOT NULL AND previous_payload IN ('true', 'false')))
    ) WITHOUT ROWID"),
    ("table", "classification_operation_targets", "CREATE TABLE classification_operation_targets (
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('category', 'review')),
        PRIMARY KEY(operation_id, record_id, field)
    ) WITHOUT ROWID"),
    ("table", "classification_review_bookmark", "CREATE TABLE classification_review_bookmark (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        scope TEXT NOT NULL CHECK(json_valid(scope) AND length(CAST(scope AS BLOB)) <= 8192),
        admission_id INTEGER NOT NULL CHECK(admission_id > 0),
        root_id TEXT NOT NULL,
        location_id TEXT NOT NULL,
        primary_missing INTEGER NOT NULL CHECK(primary_missing IN (0, 1)),
        primary_text TEXT NOT NULL CHECK(length(primary_text) <= 32768),
        primary_number INTEGER NOT NULL,
        after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)),
        revision INTEGER NOT NULL CHECK(revision > 0)
    )"),
    ("table", "classification_review_session", "CREATE TABLE classification_review_session (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        session_id TEXT NOT NULL CHECK(length(session_id) BETWEEN 1 AND 128),
        position_sequence INTEGER NOT NULL CHECK(position_sequence >= 0),
        scope TEXT NOT NULL CHECK(json_valid(scope) AND length(CAST(scope AS BLOB)) <= 8192),
        checkpoint_digest TEXT CHECK(checkpoint_digest IS NULL OR length(checkpoint_digest) = 64)
    )"),
    ("table", "classification_review_operations", "CREATE TABLE classification_review_operations (
        operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
        session_id TEXT NOT NULL CHECK(length(session_id) BETWEEN 1 AND 128),
        position_sequence INTEGER NOT NULL CHECK(position_sequence > 0),
        committed_sequence INTEGER NOT NULL CHECK(committed_sequence = position_sequence + 1)
    )"),
];
