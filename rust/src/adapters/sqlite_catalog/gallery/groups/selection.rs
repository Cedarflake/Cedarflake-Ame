use rusqlite::{Connection, params_from_iter};

use crate::domain::gallery_groups::GalleryGroupFilter;
use crate::domain::organization::stale_selection;
use crate::domain::{GalleryQuery, ScanError};

use super::super::super::database_error;
use super::super::BuiltGalleryQuery;

#[derive(Clone, Copy)]
pub(in super::super::super) enum Input {
    Organization,
    Files,
}

impl Input {
    fn table(self) -> &'static str {
        match self {
            Self::Organization => "organization_batch_ids",
            Self::Files => "file_operation_selection_input",
        }
    }
}

pub(in super::super::super) fn selected_members(
    query: &GalleryQuery,
    input: Input,
    all_matching: bool,
) -> BuiltGalleryQuery {
    let mut parameters = Vec::new();
    let members = super::member_query(query, &mut parameters);
    let count = match query.group_filter {
        GalleryGroupFilter::Similar => "COUNT(DISTINCT content_key)",
        _ => "COUNT(*)",
    };
    let membership = if all_matching { "NOT EXISTS" } else { "EXISTS" };
    BuiltGalleryQuery {
        sql: format!(
            "WITH members AS ({members}), visible_groups AS (
               SELECT entry_id FROM members GROUP BY entry_id HAVING {count} > 1
             )
             SELECT members.* FROM members JOIN visible_groups USING(entry_id)
             WHERE {membership}(SELECT 1 FROM temp.{} AS selected
               WHERE selected.asset_id = members.entry_id)",
            input.table()
        ),
        parameters,
    }
}

pub(in super::super::super) fn require_inputs(
    connection: &Connection,
    query: &GalleryQuery,
    input: Input,
) -> Result<(), ScanError> {
    let selected = selected_members(query, input, false);
    let missing: bool = connection
        .query_row(
            &format!(
                "WITH current_members AS ({}) SELECT EXISTS(
           SELECT 1 FROM temp.{} AS requested WHERE NOT EXISTS(
             SELECT 1 FROM current_members WHERE entry_id = requested.asset_id))",
                selected.sql,
                input.table()
            ),
            params_from_iter(selected.parameters),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if missing {
        Err(stale_selection())
    } else {
        Ok(())
    }
}
