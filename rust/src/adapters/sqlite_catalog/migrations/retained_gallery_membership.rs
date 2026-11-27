use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

const TABLE: &str = "CREATE TABLE gallery_retained_fingerprints (
    source_generation INTEGER NOT NULL CHECK(source_generation > 0),
    evidence_source_generation INTEGER NOT NULL,
    scheme TEXT NOT NULL,
    PRIMARY KEY(source_generation, scheme),
    CHECK(source_generation <> evidence_source_generation),
    FOREIGN KEY(evidence_source_generation, scheme)
      REFERENCES gallery_content_fingerprints(source_generation, scheme) ON DELETE CASCADE
)";
const INDEX: &str = "CREATE INDEX gallery_retained_fingerprint_evidence
    ON gallery_retained_fingerprints(evidence_source_generation, scheme)";
const VIEW: &str = "CREATE VIEW gallery_display_fingerprints AS
    SELECT f.source_generation, f.source_generation AS evidence_source_generation,
           f.scheme, f.digest, f.byte_length, f.file_identity_scheme, f.file_identity_value,
           f.source_revision_token, 0 AS is_pending
    FROM content_fingerprints AS f
    JOIN gallery_content_fingerprints AS published
      ON published.source_generation = f.source_generation AND published.scheme = f.scheme
    UNION ALL
    SELECT retained.source_generation, f.source_generation,
           f.scheme, f.digest, f.byte_length, f.file_identity_scheme, f.file_identity_value,
           f.source_revision_token, 1
    FROM gallery_retained_fingerprints AS retained
    JOIN content_fingerprints AS f
      ON f.source_generation = retained.evidence_source_generation AND f.scheme = retained.scheme
    WHERE NOT EXISTS (
      SELECT 1 FROM gallery_content_fingerprints AS current
      WHERE current.source_generation = retained.source_generation AND current.scheme = retained.scheme
    )";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in [
        ("table", "gallery_retained_fingerprints", TABLE),
        ("index", "gallery_retained_fingerprint_evidence", INDEX),
        ("view", "gallery_display_fingerprints", VIEW),
    ] {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(ScanError::new(
                "catalog_retained_gallery_membership_invalid",
                "Retained gallery membership requires its guarded display projection",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
