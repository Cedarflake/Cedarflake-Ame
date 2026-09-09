use rusqlite::{Connection, params};

use crate::domain::{LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError};

use super::{
    database_error, metadata_inventory_recovery_affinity_change_id, sqlite_integer,
    validate_policy, validate_root_id,
};

pub(super) struct ReadinessQuery<'a> {
    connection: &'a Connection,
    root_id: &'a str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    max_attempts: i64,
}

impl<'a> ReadinessQuery<'a> {
    pub(super) fn new(
        connection: &'a Connection,
        root_id: &'a str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Self, ScanError> {
        validate_policy(policy)?;
        validate_root_id(root_id)?;
        sqlite_integer(root_generation.value(), "root generation")?;
        Ok(Self {
            connection,
            root_id,
            root_generation,
            now_unix_ms,
            max_attempts: i64::from(policy.max_attempts),
        })
    }

    pub(super) fn has_ready_live_path_library_change(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND lane.lane = 'p0_live'
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(super) fn has_ready_journal_path_library_change(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND lane.lane = 'p1_journal'
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(super) fn has_ready_metadata_inventory_recovery_candidates(
        &self,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   JOIN library_metadata_inventory_candidate_owners AS owner
                     ON owner.change_id = queue.id
                   JOIN library_recovery_authorities AS authority
                     ON authority.run_id = owner.run_id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND lane.lane = 'p2_recovery'
                     AND authority.root_id = queue.root_id
                     AND authority.root_generation = queue.root_generation
                     AND authority.retired_unix_ms IS NULL
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(super) fn has_ready_legacy_unowned_recovery_debt(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND queue.scope = 'path'
                     AND queue.intent_kind <> 'freshness_unknown'
                     AND lane.lane = 'p2_recovery'
                     AND NOT EXISTS(
                       SELECT 1
                       FROM library_metadata_inventory_candidate_owners AS owner
                       WHERE owner.change_id = queue.id
                     )
                     AND NOT EXISTS(
                       SELECT 1 FROM library_recovery_authorities AS authority
                       WHERE authority.change_id = queue.id
                         AND authority.retired_unix_ms IS NULL
                     )
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 ) OR (
                   SELECT COUNT(*) > 1 OR COALESCE(MAX(
                     CASE
                       WHEN queue.status = 'retry_wait'
                         AND queue.attempt_count >= ?3
                         AND queue.next_retry_unix_ms IS NULL
                         AND queue.last_failure_code = 'legacy_recovery_authority_missing'
                       THEN 0
                       ELSE 1
                     END
                   ), 0) = 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.scope <> 'path'
                     AND lane.lane = 'p2_recovery'
                     AND queue.status IN ('pending', 'retry_wait')
                     AND NOT EXISTS(
                       SELECT 1
                       FROM library_metadata_inventory_candidate_owners AS owner
                       WHERE owner.change_id = queue.id
                     )
                     AND NOT EXISTS(
                       SELECT 1 FROM library_recovery_authorities AS authority
                       WHERE authority.change_id = queue.id
                         AND authority.retired_unix_ms IS NULL
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(super) fn has_ready_metadata_inventory_recovery(&self) -> Result<bool, ScanError> {
        let affinity_change_id = metadata_inventory_recovery_affinity_change_id(
            self.connection,
            self.root_id,
            self.root_generation,
        )?;
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                   JOIN library_recovery_authorities AS authority
                     ON authority.change_id = queue.id
                    AND authority.root_id = queue.root_id
                    AND authority.root_generation = queue.root_generation
                   LEFT JOIN library_persistent_journal_baselines AS window
                     ON window.change_id = authority.change_id
                    AND window.phase <> 'completed'
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND (?5 IS NULL OR queue.id = ?5)
                     AND lanes.lane = 'p2_recovery'
                     AND authority.retired_unix_ms IS NULL
                     AND authority.reason <> 'first_import_boundary'
                     AND authority.run_id <> ''
                     AND (authority.reason = 'watcher_uncovered_gap'
                       OR window.change_id IS NOT NULL)
                     AND (queue.scope <> 'path'
                       OR queue.intent_kind = 'freshness_unknown')
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                    affinity_change_id,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(super) fn has_ready_live_authoritative_library_change(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue AS queue
                   JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                   WHERE queue.root_id = ?1 AND queue.root_generation = ?2
                     AND queue.status IN ('pending', 'leased', 'retry_wait')
                     AND lane.lane = 'p0_live'
                     AND NOT EXISTS(
                       SELECT 1 FROM library_live_gap_recovery_claims AS claim
                       WHERE claim.gap_change_id = queue.id
                         AND claim.consumer_kind = 'pending_journal'
                     )
                     AND (queue.scope <> 'path'
                       OR queue.intent_kind = 'freshness_unknown')
                     AND (
                       (queue.attempt_count < ?3 AND queue.status = 'pending'
                         AND queue.ready_unix_ms <= ?4)
                       OR (queue.attempt_count < ?3 AND queue.status = 'retry_wait'
                         AND queue.next_retry_unix_ms IS NOT NULL
                         AND queue.next_retry_unix_ms <= ?4)
                       OR (queue.status = 'leased'
                         AND queue.lease_expires_unix_ms IS NOT NULL
                         AND queue.lease_expires_unix_ms <= ?4)
                     )
                 )",
                params![
                    self.root_id,
                    sqlite_integer(self.root_generation.value(), "root generation")?,
                    self.max_attempts,
                    self.now_unix_ms,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }
}

pub(super) fn has_unresolved_metadata_inventory_recovery_candidates(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<bool, ScanError> {
    validate_root_id(root_id)?;
    connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_candidate_owners AS owner
               JOIN library_recovery_authorities AS authority
                 ON authority.run_id = owner.run_id
               JOIN library_change_queue AS queue ON queue.id = owner.change_id
               WHERE authority.root_id = ?1 AND authority.root_generation = ?2
                 AND authority.retired_unix_ms IS NULL
                 AND queue.status <> 'completed'
             )",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}
