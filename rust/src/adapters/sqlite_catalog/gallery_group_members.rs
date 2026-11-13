use rusqlite::{params_from_iter, types::Value};

use crate::domain::gallery_groups::{
    GalleryGroupFilter, GalleryGroupKey, GalleryGroupMember, GalleryGroupMemberPage,
};
use crate::domain::similar_images::SimilarContentKey;
use crate::domain::{GalleryQuery, ScanError};

use super::{SqliteCatalog, database_error, read_stored_asset, sqlite_unsigned, stored_asset_view};

impl SqliteCatalog {
    pub(super) fn load_gallery_group_members(
        &self,
        query: &GalleryQuery,
        group: &GalleryGroupKey,
        after: &str,
    ) -> Result<GalleryGroupMemberPage, ScanError> {
        let inspect_all = query.group_filter == GalleryGroupFilter::Unrestricted;
        let mut query = query.clone();
        query.group_filter = match group {
            GalleryGroupKey::Exact { .. } => GalleryGroupFilter::Exact,
            GalleryGroupKey::Similar { .. } => GalleryGroupFilter::Similar,
        };
        super::gallery::validate_gallery_query(&query)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let decision_revision = if let GalleryGroupKey::Similar { group_id } = group {
            let revision: i64 = transaction.query_row(
                "SELECT COALESCE((SELECT revision FROM similarity_decisions WHERE group_id = ?1), 0)",
                [group_id], |row| row.get(0),
            ).map_err(database_error)?;
            sqlite_unsigned(revision, "similar decision revision")?
        } else {
            0
        };
        let mut parameters = Vec::new();
        let members = if inspect_all {
            super::gallery::groups::inspection_members(&query, &mut parameters)
        } else {
            super::gallery::groups::member_query(&query, &mut parameters)
        };
        parameters.push(Value::Text(group.entry_id()));
        let source = format!("FROM ({members}) AS locations WHERE entry_id = ?");
        let (total, pending): (i64, i64) = transaction
            .query_row(
                &format!("SELECT COUNT(*), COALESCE(SUM(is_pending), 0) {source}"),
                params_from_iter(&parameters),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(database_error)?;
        parameters.push(Value::Text(after.to_owned()));
        let mut statement = transaction
            .prepare(&format!(
                "SELECT {}, content_scheme, content_digest, content_bytes, preferred
             {source} AND location_id > ? ORDER BY location_id LIMIT 81",
                super::gallery::ASSET_COLUMNS,
            ))
            .map_err(database_error)?;
        let mut rows = statement
            .query(params_from_iter(parameters))
            .map_err(database_error)?;
        let mut members = Vec::new();
        while let Some(row) = rows.next().map_err(database_error)? {
            members.push(GalleryGroupMember {
                availability:
                    crate::domain::exact_duplicates::DuplicateLocationAvailability::Unavailable,
                source: stored_asset_view(read_stored_asset(row).map_err(database_error)?)?,
                content: SimilarContentKey {
                    scheme: row.get("content_scheme").map_err(database_error)?,
                    digest: row.get("content_digest").map_err(database_error)?,
                    byte_length: sqlite_unsigned(
                        row.get("content_bytes").map_err(database_error)?,
                        "member bytes",
                    )?,
                },
                preferred: row.get("preferred").map_err(database_error)?,
            });
        }
        let next_location_id = if members.len() > 80 {
            members.truncate(80);
            members
                .last()
                .map(|member| member.source.location_id.clone())
        } else {
            None
        };
        Ok(GalleryGroupMemberPage {
            members,
            total: sqlite_unsigned(total, "group members")?,
            pending: sqlite_unsigned(pending, "pending group members")?,
            next_location_id,
            decision_revision,
        })
    }
}
