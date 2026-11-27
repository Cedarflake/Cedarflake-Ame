pub(super) const DECISIONS: &str = "CREATE TABLE \"similarity_decisions\" (
    group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision >= 0)
) STRICT, WITHOUT ROWID";

pub(super) const PREFERENCES: &str = "CREATE TABLE similarity_preferences (
    group_id TEXT NOT NULL REFERENCES similarity_groups(group_id),
    fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
    PRIMARY KEY(group_id, fingerprint_scheme, digest, byte_length)
) STRICT, WITHOUT ROWID";
