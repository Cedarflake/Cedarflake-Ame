pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "similarity_runs", "CREATE TABLE similarity_runs (
        run_id TEXT PRIMARY KEY NOT NULL CHECK(length(run_id) BETWEEN 1 AND 128),
        source_run_id TEXT NOT NULL REFERENCES exact_analysis_runs(run_id),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        engine_id TEXT NOT NULL CHECK(length(engine_id) BETWEEN 1 AND 128),
        engine_version TEXT NOT NULL CHECK(length(engine_version) BETWEEN 1 AND 128),
        parameters TEXT NOT NULL CHECK(length(parameters) BETWEEN 1 AND 4096),
        started_unix_ms INTEGER NOT NULL CHECK(started_unix_ms >= 0)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_evidence", "CREATE TABLE similarity_evidence (
        evidence_id INTEGER PRIMARY KEY NOT NULL CHECK(evidence_id > 0),
        run_id TEXT NOT NULL REFERENCES similarity_runs(run_id),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
        disposition TEXT NOT NULL CHECK(disposition IN ('ready', 'insufficient_structure', 'decode_failed')),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        UNIQUE(scheme, fingerprint_scheme, digest, byte_length),
        CHECK((disposition = 'decode_failed') = (issue_code IS NOT NULL))
    ) STRICT"),
    ("table", "similarity_variants", "CREATE TABLE similarity_variants (
        evidence_id INTEGER NOT NULL REFERENCES similarity_evidence(evidence_id),
        variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
        word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
        word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
        word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
        word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
        PRIMARY KEY(evidence_id, variant),
        UNIQUE(evidence_id, word0, word1, word2, word3)
    ) STRICT, WITHOUT ROWID"),
    ("index", "similarity_variants_band0", "CREATE INDEX similarity_variants_band0 ON similarity_variants(word0, evidence_id, variant)"),
    ("index", "similarity_variants_band1", "CREATE INDEX similarity_variants_band1 ON similarity_variants(word1, evidence_id, variant)"),
    ("index", "similarity_variants_band2", "CREATE INDEX similarity_variants_band2 ON similarity_variants(word2, evidence_id, variant)"),
    ("index", "similarity_variants_band3", "CREATE INDEX similarity_variants_band3 ON similarity_variants(word3, evidence_id, variant)"),
    ("table", "similarity_groups", "CREATE TABLE similarity_groups (
        group_id TEXT PRIMARY KEY NOT NULL CHECK(length(group_id) = 64 AND group_id NOT GLOB '*[^0-9a-f]*'),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        reference_scheme TEXT NOT NULL CHECK(length(reference_scheme) BETWEEN 1 AND 128),
        reference_digest TEXT NOT NULL CHECK(length(reference_digest) = 64 AND reference_digest NOT GLOB '*[^0-9a-f]*'),
        reference_byte_length INTEGER NOT NULL CHECK(reference_byte_length BETWEEN 1 AND 268435456),
        UNIQUE(scheme, reference_scheme, reference_digest, reference_byte_length)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_references", "CREATE TABLE similarity_references (
        group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
        evidence_id INTEGER NOT NULL UNIQUE REFERENCES similarity_evidence(evidence_id)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_reference_variants", "CREATE TABLE similarity_reference_variants (
        group_id TEXT NOT NULL REFERENCES similarity_references(group_id),
        variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
        word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
        word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
        word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
        word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
        PRIMARY KEY(group_id, variant)
    ) STRICT, WITHOUT ROWID"),
    ("index", "similarity_references_band0", "CREATE INDEX similarity_references_band0 ON similarity_reference_variants(word0, group_id, variant)"),
    ("index", "similarity_references_band1", "CREATE INDEX similarity_references_band1 ON similarity_reference_variants(word1, group_id, variant)"),
    ("index", "similarity_references_band2", "CREATE INDEX similarity_references_band2 ON similarity_reference_variants(word2, group_id, variant)"),
    ("index", "similarity_references_band3", "CREATE INDEX similarity_references_band3 ON similarity_reference_variants(word3, group_id, variant)"),
    ("table", "similarity_members", "CREATE TABLE similarity_members (
        group_id TEXT NOT NULL REFERENCES similarity_references(group_id),
        evidence_id INTEGER NOT NULL REFERENCES similarity_evidence(evidence_id),
        distance INTEGER NOT NULL CHECK(distance BETWEEN 0 AND 10),
        run_id TEXT NOT NULL REFERENCES similarity_runs(run_id),
        PRIMARY KEY(group_id, evidence_id)
    ) STRICT, WITHOUT ROWID"),
    ("index", "similarity_members_content", "CREATE INDEX similarity_members_content ON similarity_members(evidence_id, group_id)"),
    ("table", "similarity_decisions", "CREATE TABLE similarity_decisions (
        group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
        preferred_scheme TEXT,
        preferred_digest TEXT CHECK(preferred_digest IS NULL OR (length(preferred_digest) = 64 AND preferred_digest NOT GLOB '*[^0-9a-f]*')),
        preferred_byte_length INTEGER CHECK(preferred_byte_length IS NULL OR preferred_byte_length BETWEEN 1 AND 268435456),
        ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        CHECK((preferred_scheme IS NULL) = (preferred_digest IS NULL)),
        CHECK((preferred_scheme IS NULL) = (preferred_byte_length IS NULL)),
        CHECK(preferred_scheme IS NULL OR length(preferred_scheme) BETWEEN 1 AND 128)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_exclusions", "CREATE TABLE similarity_exclusions (
        group_id TEXT NOT NULL REFERENCES similarity_groups(group_id),
        fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
        exclusion_id TEXT NOT NULL UNIQUE CHECK(length(exclusion_id) BETWEEN 1 AND 128),
        PRIMARY KEY(group_id, fingerprint_scheme, digest, byte_length)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_source_failures", "CREATE TABLE similarity_source_failures (
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        root_id TEXT NOT NULL,
        root_generation INTEGER NOT NULL CHECK(root_generation > 0),
        location_id TEXT NOT NULL,
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        issue_code TEXT NOT NULL CHECK(length(issue_code) BETWEEN 1 AND 128),
        retry_unix_ms INTEGER CHECK(retry_unix_ms IS NULL OR retry_unix_ms >= 0),
        PRIMARY KEY(source_generation, root_id, root_generation, location_id, scheme)
    ) STRICT, WITHOUT ROWID"),
    ("table", "similarity_match_work", "CREATE TABLE similarity_match_work (
        evidence_id INTEGER PRIMARY KEY NOT NULL REFERENCES similarity_evidence(evidence_id),
        phase TEXT NOT NULL CHECK(phase IN ('references', 'peer', 'members', 'complete')),
        reference_revision INTEGER NOT NULL CHECK(reference_revision >= 0),
        query_variant INTEGER NOT NULL CHECK(query_variant BETWEEN 0 AND 7),
        band INTEGER NOT NULL CHECK(band BETWEEN 0 AND 3),
        neighbor INTEGER NOT NULL CHECK(neighbor BETWEEN 0 AND 136),
        after_evidence_id INTEGER NOT NULL CHECK(after_evidence_id >= 0),
        after_group_id TEXT NOT NULL CHECK(length(after_group_id) <= 64),
        after_variant INTEGER NOT NULL CHECK(after_variant BETWEEN -1 AND 7),
        found_reference INTEGER NOT NULL CHECK(found_reference IN (0, 1))
    ) STRICT"),
    ("table", "similarity_dispatch", "CREATE TABLE similarity_dispatch (
        singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
        active_run_id TEXT REFERENCES similarity_runs(run_id),
        lease_until_unix_ms INTEGER,
        reference_revision INTEGER NOT NULL CHECK(reference_revision >= 0),
        CHECK((active_run_id IS NULL) = (lease_until_unix_ms IS NULL)),
        CHECK(lease_until_unix_ms IS NULL OR lease_until_unix_ms >= 0)
    ) STRICT"),
    ("table", "similarity_outcomes", "CREATE TABLE similarity_outcomes (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_runs(run_id),
        phase TEXT NOT NULL CHECK(phase IN ('completed', 'cancelled', 'failed')),
        checked_sources INTEGER NOT NULL CHECK(checked_sources >= 0),
        analyzed_contents INTEGER NOT NULL CHECK(analyzed_contents >= 0),
        skipped_sources INTEGER NOT NULL CHECK(skipped_sources >= 0),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        finished_unix_ms INTEGER NOT NULL CHECK(finished_unix_ms >= 0)
    ) STRICT, WITHOUT ROWID"),
    ("index", "similarity_work_pending", "CREATE INDEX similarity_work_pending ON similarity_match_work(phase, evidence_id)"),
    ("index", "similarity_source_retries", "CREATE INDEX similarity_source_retries ON similarity_source_failures(retry_unix_ms) WHERE retry_unix_ms IS NOT NULL"),
];
