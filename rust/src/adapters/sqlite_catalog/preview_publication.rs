use super::*;

/// Publishes the artifact and its source-bound metadata in one catalog transaction.
pub(super) fn publish(
    catalog: &mut SqliteCatalog,
    location: &AssetLocationView,
    artifact: Option<&PreviewArtifact>,
    request: Option<&PreviewRequest>,
    authority: Option<&crate::ports::PreviewPublicationAuthority>,
) -> Result<(), ScanError> {
    let publication = PreviewLocationPublication::new(location, request, authority)?;
    let transaction = catalog.begin_write()?;
    publication.validate_root(&transaction)?;
    publish_artifact(&transaction, location, artifact, publication.file_size)?;
    publication.publish_location(&transaction)?;
    transaction.commit().map_err(database_error)
}

struct PreviewLocationPublication<'a> {
    location: &'a AssetLocationView,
    file_size: i64,
    expected_scan_id: &'a str,
    expected_source_generation: u64,
    expected_source_revision: Option<String>,
    published_source_revision: Option<String>,
    expected_root_generation: Option<i64>,
    expected_root_identity_scheme: Option<&'a str>,
    expected_root_identity_value: Option<&'a str>,
}

impl<'a> PreviewLocationPublication<'a> {
    fn new(
        location: &'a AssetLocationView,
        request: Option<&'a PreviewRequest>,
        publication_authority: Option<&'a crate::ports::PreviewPublicationAuthority>,
    ) -> Result<Self, ScanError> {
        if let Some(request) = request
            && (request.location_id != location.location_id
                || request.expected_root_id != location.root_id
                || request.expected_scan_id != location.scan_id
                || request.expected_source_generation != location.source_generation
                || (request.expected_source_revision.is_some()
                    && request.expected_source_revision != location.source_revision))
        {
            return Err(ScanError::new(
                "preview_request_context_invalid",
                "The preview result does not belong to its requested source context",
            ));
        }
        let file_size = sqlite_integer(location.file_size, "file size")?;
        let expected_scan_id = request.map_or(location.scan_id.as_str(), |request| {
            request.expected_scan_id.as_str()
        });
        let expected_source_generation = request.map_or(location.source_generation, |request| {
            request.expected_source_generation
        });
        let expected_source_revision = request
            .map_or(location.source_revision.as_ref(), |request| {
                request.expected_source_revision.as_ref()
            })
            .map(source_revision_token)
            .transpose()?;
        let published_source_revision = location
            .source_revision
            .as_ref()
            .map(source_revision_token)
            .transpose()?;
        let expected_root_generation = publication_authority
            .map(|authority| sqlite_integer(authority.root_generation.value(), "root generation"))
            .transpose()?;
        let expected_root_identity_scheme = publication_authority
            .and_then(|authority| authority.root_identity.as_ref())
            .map(|identity| identity.scheme.as_str());
        let expected_root_identity_value = publication_authority
            .and_then(|authority| authority.root_identity.as_ref())
            .map(|identity| identity.value.as_str());
        Ok(Self {
            location,
            file_size,
            expected_scan_id,
            expected_source_generation,
            expected_source_revision,
            published_source_revision,
            expected_root_generation,
            expected_root_identity_scheme,
            expected_root_identity_value,
        })
    }

    fn validate_root(&self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        if self.expected_root_generation.is_some() {
            let authority_is_current = transaction
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1
                       FROM library_change_root_state AS root_state
                       WHERE root_state.root_id = ?1
                         AND root_state.generation = ?2
                         AND root_state.is_active = 1
                         AND (
                           (
                             ?3 IS NULL AND ?4 IS NULL
                             AND NOT EXISTS (
                               SELECT 1
                               FROM library_root_publication_namespaces AS namespace
                               WHERE namespace.root_id = root_state.root_id
                             )
                           )
                           OR EXISTS (
                             SELECT 1
                             FROM library_root_publication_namespaces AS namespace
                             WHERE namespace.root_id = root_state.root_id
                               AND namespace.root_generation = root_state.generation
                               AND namespace.identity_scheme = ?3
                               AND namespace.identity_value = ?4
                           )
                         )
                     )",
                    params![
                        self.location.root_id,
                        self.expected_root_generation,
                        self.expected_root_identity_scheme,
                        self.expected_root_identity_value,
                    ],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if !authority_is_current {
                return Err(ScanError::new(
                    "active_preview_authority_stale",
                    "The library root publication authority changed before its preview was updated",
                ));
            }
        }
        Ok(())
    }

    fn publish_location(self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        let Self {
            location,
            file_size,
            expected_scan_id,
            expected_source_generation,
            expected_source_revision,
            published_source_revision,
            expected_root_generation,
            expected_root_identity_scheme,
            expected_root_identity_value,
        } = self;
        let changes_gallery_order = transaction
            .query_row(
                "SELECT COALESCE(capture_local_time, file_local_time)
                          IS NOT COALESCE(?3, file_local_time)
                 FROM asset_locations
                 WHERE scan_id = ?1 AND location_id = ?2",
                params![
                    expected_scan_id,
                    location.location_id,
                    location
                        .capture_time
                        .as_ref()
                        .map(|evidence| &evidence.local_time)
                ],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(database_error)?
            .unwrap_or(false);
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = ?2, width = ?3, height = ?4,
                      preview_status = ?5, preview_issue_code = ?6,
                      preview_issue_message = ?7, source_revision_token = ?14,
                      metadata_engine_id = ?21, metadata_engine_version = ?22,
                      capture_local_time = ?23, capture_offset_minutes = ?24,
                      capture_time_source = ?25, capture_raw_value = ?26
                 WHERE location_id = ?1 AND file_size = ?8 AND modified_unix_ms = ?9
                    AND root_id = ?10 AND absolute_path = ?11
                    AND file_identity_scheme IS ?12 AND file_identity_value IS ?13
                    AND scan_id = ?15 AND source_generation = ?16
                    AND (
                      source_revision_token IS ?17
                      OR (?17 IS NULL AND source_revision_token = ?14)
                    )
                    AND (
                      ?14 IS NULL OR file_identity_scheme IS NULL OR NOT EXISTS (
                        SELECT 1
                        FROM asset_locations AS sibling
                        JOIN library_roots AS sibling_root
                          ON sibling_root.id = sibling.root_id
                         AND sibling_root.active_scan_id = sibling.scan_id
                        WHERE sibling.file_identity_scheme = asset_locations.file_identity_scheme
                          AND sibling.file_identity_value = asset_locations.file_identity_value
                          AND sibling.source_generation = ?16
                          AND sibling.source_revision_token IS NOT NULL
                          AND sibling.source_revision_token <> ?14
                      )
                    )
                    AND scan_id = (
                      SELECT active_scan_id FROM library_roots WHERE id = ?10
                    )
                    AND (
                      ?18 IS NULL OR (
                        EXISTS (
                          SELECT 1
                          FROM library_change_root_state AS root_state
                          WHERE root_state.root_id = ?10
                            AND root_state.generation = ?18
                            AND root_state.is_active = 1
                        )
                        AND (
                          (
                            ?19 IS NULL AND ?20 IS NULL
                            AND NOT EXISTS (
                              SELECT 1
                              FROM library_root_publication_namespaces AS namespace
                              WHERE namespace.root_id = ?10
                            )
                          )
                          OR EXISTS (
                            SELECT 1
                            FROM library_root_publication_namespaces AS namespace
                            WHERE namespace.root_id = ?10
                              AND namespace.root_generation = ?18
                              AND namespace.identity_scheme = ?19
                              AND namespace.identity_value = ?20
                          )
                        )
                      )
                    )",
                params![
                    location.location_id,
                    location.preview_path,
                    i64::from(location.width),
                    i64::from(location.height),
                    preview_status_text(&location.preview_status),
                    location.preview_issue_code,
                    location.preview_issue_message,
                    file_size,
                    location.modified_unix_ms,
                    location.root_id,
                    location.absolute_path,
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.scheme),
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.value),
                    published_source_revision,
                    expected_scan_id,
                    sqlite_integer(expected_source_generation, "source generation")?,
                    expected_source_revision,
                    expected_root_generation,
                    expected_root_identity_scheme,
                    expected_root_identity_value,
                    location.metadata_engine_id,
                    location.metadata_engine_version,
                    location
                        .capture_time
                        .as_ref()
                        .map(|evidence| &evidence.local_time),
                    location
                        .capture_time
                        .as_ref()
                        .and_then(|evidence| evidence.offset_minutes)
                        .map(i64::from),
                    location
                        .capture_time
                        .as_ref()
                        .map(|evidence| capture_time_source_text(&evidence.source)),
                    location
                        .capture_time
                        .as_ref()
                        .map(|evidence| &evidence.raw_value),
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "active_preview_location_stale",
                "The active catalog location changed before its preview was updated",
            ));
        }
        if let (Some(identity), Some(revision)) = (
            location.file_identity.as_ref(),
            published_source_revision.as_ref(),
        ) {
            transaction
                .execute(
                    "UPDATE asset_locations
                     SET source_revision_token = ?3
                     WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
                       AND source_generation = ?4 AND source_revision_token IS NULL
                       AND EXISTS (
                         SELECT 1 FROM library_roots AS roots
                         WHERE roots.id = asset_locations.root_id
                           AND roots.active_scan_id = asset_locations.scan_id
                       )",
                    params![
                        identity.scheme,
                        identity.value,
                        revision,
                        sqlite_integer(location.source_generation, "source generation")?,
                    ],
                )
                .map_err(database_error)?;
        }
        if changes_gallery_order {
            let updated = transaction
                .execute("UPDATE catalog_state SET revision = revision + 1", [])
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_revision_unavailable",
                    "The catalog revision state is missing or invalid",
                ));
            }
        }
        Ok(())
    }
}

fn publish_artifact(
    transaction: &Transaction<'_>,
    location: &AssetLocationView,
    artifact: Option<&PreviewArtifact>,
    file_size: i64,
) -> Result<(), ScanError> {
    if let Some(artifact) = artifact {
        if location.source_revision.is_none() || location.source_generation == 0 {
            return Err(ScanError::new(
                "preview_source_revision_unproven",
                "A preview cannot be published before the source revision is proven",
            ));
        }
        let artifact_bytes = sqlite_integer(artifact.byte_size, "preview artifact size")?;
        transaction
            .execute(
                "DELETE FROM preview_artifact_locations
                 WHERE location_id = ?1
                   AND artifact_key IN (
                     SELECT artifact_key FROM preview_artifacts
                     WHERE algorithm_id = ?2
                       AND orientation_contract = ?3
                       AND size_bucket = ?4
                       AND artifact_key <> ?5
                   )",
                params![
                    location.location_id,
                    artifact.algorithm_id,
                    artifact.orientation_contract,
                    i64::from(artifact.size_bucket),
                    artifact.artifact_key,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE preview_artifacts
                 SET lifecycle_state = 'stale'
                 WHERE lifecycle_state = 'ready'
                   AND algorithm_id = ?1
                   AND orientation_contract = ?2
                   AND size_bucket = ?3
                   AND artifact_key <> ?4
                   AND NOT EXISTS (
                     SELECT 1 FROM preview_artifact_locations AS owners
                     WHERE owners.artifact_key = preview_artifacts.artifact_key
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                     WHERE handoffs.preview_status = 'ready'
                       AND handoffs.preview_path = preview_artifacts.artifact_path
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                     WHERE handoffs.preview_status = 'ready'
                       AND handoffs.preview_path = preview_artifacts.artifact_path
                   )",
                params![
                    artifact.algorithm_id,
                    artifact.orientation_contract,
                    i64::from(artifact.size_bucket),
                    artifact.artifact_key,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO preview_artifacts(
                   artifact_key, source_file_size, source_modified_unix_ms,
                   source_identity_scheme, source_identity_value, source_revision_token,
                   source_generation, algorithm_id,
                   algorithm_version, orientation_contract, size_bucket, encoded_width,
                   encoded_height, artifact_path, byte_size, lifecycle_state,
                   created_unix_ms, last_used_unix_ms
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, ?15, 'ready', ?16, ?16
                 )
                 ON CONFLICT(artifact_key) DO UPDATE SET
                   source_file_size = excluded.source_file_size,
                   source_modified_unix_ms = excluded.source_modified_unix_ms,
                   source_identity_scheme = excluded.source_identity_scheme,
                   source_identity_value = excluded.source_identity_value,
                   source_revision_token = excluded.source_revision_token,
                   source_generation = excluded.source_generation,
                   algorithm_id = excluded.algorithm_id,
                   algorithm_version = excluded.algorithm_version,
                   orientation_contract = excluded.orientation_contract,
                   size_bucket = excluded.size_bucket,
                   encoded_width = excluded.encoded_width,
                   encoded_height = excluded.encoded_height,
                   artifact_path = excluded.artifact_path,
                   byte_size = excluded.byte_size,
                   lifecycle_state = 'ready',
                   last_used_unix_ms = excluded.last_used_unix_ms",
                params![
                    artifact.artifact_key,
                    file_size,
                    location.modified_unix_ms,
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.scheme),
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.value),
                    location
                        .source_revision
                        .as_ref()
                        .map(source_revision_token)
                        .transpose()?,
                    sqlite_integer(location.source_generation, "source generation")?,
                    artifact.algorithm_id,
                    i64::from(artifact.algorithm_version),
                    artifact.orientation_contract,
                    i64::from(artifact.size_bucket),
                    i64::from(artifact.encoded_width),
                    i64::from(artifact.encoded_height),
                    artifact.path,
                    artifact_bytes,
                    unix_time_ms(),
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO preview_artifact_locations(
                   artifact_key, location_id
                 ) VALUES (?1, ?2)",
                params![artifact.artifact_key, location.location_id],
            )
            .map_err(database_error)?;
    } else if !matches!(location.preview_status, PreviewStatus::Ready) {
        transaction
            .execute(
                "DELETE FROM preview_artifact_locations WHERE location_id = ?1",
                [&location.location_id],
            )
            .map_err(database_error)?;
        mark_unreferenced_preview_artifacts_stale(transaction)?;
    }
    Ok(())
}
