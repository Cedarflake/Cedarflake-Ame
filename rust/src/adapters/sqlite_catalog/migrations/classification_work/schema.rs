pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "classification_dispatch_state", "CREATE TABLE classification_dispatch_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        epoch INTEGER NOT NULL CHECK(epoch >= 0),
        mode TEXT NOT NULL CHECK(mode IN ('disabled', 'automatic', 'paused', 'stopped')),
        run_id TEXT REFERENCES classification_runs(run_id),
        session_id TEXT CHECK(session_id IS NULL OR length(session_id) BETWEEN 1 AND 128),
        source_cursor INTEGER NOT NULL CHECK(source_cursor >= 0),
        lease_clock INTEGER NOT NULL CHECK(lease_clock >= 0),
        fairness_cursor INTEGER NOT NULL CHECK(fairness_cursor BETWEEN 0 AND 6),
        CHECK(mode = 'disabled' OR run_id IS NOT NULL)
    ) STRICT"),
    ("table", "classification_stopped_runs", "CREATE TABLE classification_stopped_runs (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_runs(run_id)
    ) STRICT, WITHOUT ROWID"),
    ("table", "classification_work_items", "CREATE TABLE classification_work_items (
        work_id INTEGER PRIMARY KEY,
        created_run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
        configuration_revision TEXT NOT NULL CHECK(length(configuration_revision) = 64),
        kind TEXT NOT NULL CHECK(kind IN ('source', 'content')),
        source_generation INTEGER,
        fingerprint_scheme TEXT,
        digest TEXT,
        byte_length INTEGER,
        priority INTEGER NOT NULL CHECK(priority BETWEEN 0 AND 2),
        phase TEXT NOT NULL CHECK(phase IN ('queued', 'running', 'waiting', 'done', 'failed', 'retired')),
        failure_count INTEGER NOT NULL CHECK(failure_count BETWEEN 0 AND 3),
        ready_unix_ms INTEGER NOT NULL CHECK(ready_unix_ms >= 0),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        lease_id INTEGER UNIQUE,
        lease_epoch INTEGER,
        lease_asset_id TEXT,
        lease_location_id TEXT,
        lease_source_generation INTEGER,
        CHECK((kind = 'source' AND source_generation IS NOT NULL AND source_generation > 0 AND fingerprint_scheme IS NULL
               AND digest IS NULL AND byte_length IS NULL)
           OR (kind = 'content' AND source_generation IS NULL AND fingerprint_scheme IS NOT NULL
               AND length(fingerprint_scheme) BETWEEN 1 AND 128 AND digest IS NOT NULL
               AND length(digest) = 64 AND byte_length IS NOT NULL AND byte_length > 0)),
        CHECK((phase = 'running' AND lease_id IS NOT NULL AND lease_id > 0
               AND lease_epoch IS NOT NULL AND lease_epoch >= 0
               AND lease_asset_id IS NOT NULL AND lease_location_id IS NOT NULL
               AND lease_source_generation IS NOT NULL
               AND length(lease_asset_id) BETWEEN 1 AND 128
               AND length(lease_location_id) BETWEEN 1 AND 128 AND lease_source_generation > 0
               AND (kind = 'content' OR lease_source_generation = source_generation))
           OR (phase <> 'running' AND lease_id IS NULL AND lease_epoch IS NULL
               AND lease_asset_id IS NULL AND lease_location_id IS NULL AND lease_source_generation IS NULL))
    ) STRICT"),
    ("index", "classification_work_source", "CREATE UNIQUE INDEX classification_work_source
        ON classification_work_items(configuration_revision, source_generation) WHERE kind = 'source'"),
    ("index", "classification_work_content", "CREATE UNIQUE INDEX classification_work_content
        ON classification_work_items(configuration_revision, fingerprint_scheme, digest, byte_length) WHERE kind = 'content'"),
    ("index", "classification_work_ready", "CREATE INDEX classification_work_ready
        ON classification_work_items(configuration_revision, phase, priority, kind, ready_unix_ms, work_id)"),
    ("index", "classification_work_single_claim", "CREATE UNIQUE INDEX classification_work_single_claim
        ON classification_work_items((1)) WHERE phase = 'running'"),
];
