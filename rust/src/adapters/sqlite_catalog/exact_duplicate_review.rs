use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicatePreference, ExactDuplicateReviewKey,
    ExactDuplicateReviewPage, ExactDuplicateReviewRange,
};
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::{ExactDuplicateGroupCursor, ScanError};
use crate::ports::exact_duplicate_review::{
    ExactDuplicateReviewMembers, ExactDuplicateReviewRepository,
};

use super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::{SqliteCatalog, database_error, sqlite_integer, unix_time_ms};

mod groups;

pub(super) const DECISION_JOIN: &str = "LEFT JOIN exact_duplicate_decisions AS decision
    ON decision.scheme = f.scheme AND decision.digest = f.digest
    AND decision.byte_length = f.byte_length";

pub(super) fn representative_expression(location: &str) -> String {
    // A removed, replaced or out-of-scope preferred location cannot change this scope's membership.
    format!(
        "COALESCE(MIN(CASE
        WHEN {location}.location_id = decision.preferred_location_id
          AND {location}.asset_id = decision.preferred_asset_id
        THEN {location}.location_id END), MIN({location}.location_id))"
    )
}

impl ExactDuplicateReviewRepository for SqliteCatalog {
    fn load_review_members(
        &self,
        key: &ExactDuplicateReviewKey,
        after: &str,
    ) -> Result<ExactDuplicateReviewMembers, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let candidates = super::exact_duplicates::queries::load_locations(
            &transaction,
            &key.digest,
            key.byte_length,
            after,
        )?;
        let decision = load_decision(&transaction, key)?;
        transaction.commit().map_err(database_error)?;
        Ok(ExactDuplicateReviewMembers {
            candidates,
            decision,
        })
    }

    fn load_review_groups(
        &self,
        after: Option<&ExactDuplicateGroupCursor>,
        ignored: bool,
    ) -> Result<ExactDuplicateReviewPage, ScanError> {
        groups::load(&self.connection, after, ignored)
    }

    fn seek_review_groups(
        &self,
        index: u64,
        ignored: bool,
    ) -> Result<ExactDuplicateReviewRange, ScanError> {
        groups::seek(&self.connection, index, ignored)
    }

    fn set_duplicate_preference(
        &mut self,
        key: &ExactDuplicateReviewKey,
        preferred: Option<&ExactDuplicatePreference>,
    ) -> Result<ExactDuplicateDecision, ScanError> {
        let transaction = self.begin_write()?;
        if let Some(preferred) = preferred {
            let current: bool = transaction
                .query_row(
                    &format!(
                        "SELECT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
                 JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                 WHERE f.digest = ?2 AND f.byte_length = ?3
                   AND l.location_id = ?4 AND l.asset_id = ?5 AND l.source_generation = ?6)"
                    ),
                    params![
                        EXACT_FINGERPRINT_SCHEME,
                        key.digest,
                        sqlite_integer(key.byte_length, "content length")?,
                        preferred.location_id,
                        preferred.asset_id,
                        sqlite_integer(preferred.source_generation, "source generation")?
                    ],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            if !current {
                return Err(stale_group());
            }
        }
        let mut decision = load_decision(&transaction, key)?;
        let location_id = preferred.map(|value| value.location_id.clone());
        let asset_id = preferred.map(|value| value.asset_id.clone());
        if decision.preferred_location_id != location_id || decision.preferred_asset_id != asset_id
        {
            decision.preferred_location_id = location_id;
            decision.preferred_asset_id = asset_id;
            save_decision(&transaction, key, &decision)?;
            transaction
                .execute("UPDATE catalog_state SET revision = revision + 1", [])
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(decision)
    }

    fn set_duplicate_ignored(
        &mut self,
        key: &ExactDuplicateReviewKey,
        ignored: bool,
    ) -> Result<ExactDuplicateDecision, ScanError> {
        let transaction = self.begin_write()?;
        if ignored {
            let count: i64 = transaction
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM (SELECT 1 FROM {CURRENT_LOCATIONS}
                 JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                 WHERE f.digest = ?2 AND f.byte_length = ?3 LIMIT 2)"
                    ),
                    params![
                        EXACT_FINGERPRINT_SCHEME,
                        key.digest,
                        sqlite_integer(key.byte_length, "content length")?
                    ],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            if count < 2 {
                return Err(stale_group());
            }
        }
        let mut decision = load_decision(&transaction, key)?;
        if decision.ignored != ignored {
            decision.ignored = ignored;
            save_decision(&transaction, key, &decision)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(decision)
    }
}

fn load_decision(
    connection: &Connection,
    key: &ExactDuplicateReviewKey,
) -> Result<ExactDuplicateDecision, ScanError> {
    connection.query_row(
        "SELECT preferred_location_id, preferred_asset_id, ignored FROM exact_duplicate_decisions
         WHERE scheme = ?1 AND digest = ?2 AND byte_length = ?3",
        params![EXACT_FINGERPRINT_SCHEME, key.digest, sqlite_integer(key.byte_length, "content length")?],
        |row| Ok(ExactDuplicateDecision { preferred_location_id: row.get(0)?, preferred_asset_id: row.get(1)?, ignored: row.get(2)? }))
        .optional().map(|value| value.unwrap_or_default()).map_err(database_error)
}

fn save_decision(
    connection: &Connection,
    key: &ExactDuplicateReviewKey,
    decision: &ExactDuplicateDecision,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT INTO exact_duplicate_decisions(scheme, digest, byte_length,
          preferred_location_id, preferred_asset_id, ignored, updated_unix_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(scheme, digest, byte_length) DO UPDATE SET
          preferred_location_id = excluded.preferred_location_id,
          preferred_asset_id = excluded.preferred_asset_id,
          ignored = excluded.ignored, updated_unix_ms = excluded.updated_unix_ms",
            params![
                EXACT_FINGERPRINT_SCHEME,
                key.digest,
                sqlite_integer(key.byte_length, "content length")?,
                decision.preferred_location_id,
                decision.preferred_asset_id,
                decision.ignored,
                unix_time_ms()
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn stale_group() -> ScanError {
    ScanError::new(
        "exact_duplicate_group_changed",
        "The selected file no longer belongs to this exact content group",
    )
}
