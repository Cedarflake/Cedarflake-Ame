use rusqlite::{Connection, OptionalExtension, params};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::file_operation_exports::{
    FileOrganizationDestination as Destination, FileOrganizationRule as Rule,
};
use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
use crate::domain::organization::OrganizationSelection;

use super::super::super::{database_error, gallery, organization, sqlite_unsigned};

pub(super) fn freeze(
    connection: &Connection,
    rule: Rule,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let mut after = 0_i64;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "file_operation_selection_closed",
                "The file organization selection was cancelled",
            ));
        }
        let mut statement = transaction
            .prepare(
                "SELECT ordinal, asset_id, location_id, source_generation
            FROM temp.file_operation_selection WHERE ordinal > ?1 ORDER BY ordinal LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![after, FILE_OPERATION_PAGE_SIZE as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        drop(statement);
        if rows.is_empty() {
            break;
        }
        for (ordinal, asset_id, location_id, generation) in rows {
            let selected = OrganizationSelection {
                asset_id,
                location_id,
                source_generation: sqlite_unsigned(generation, "selected generation")?,
            };
            let destination = resolve(&transaction, &selected, rule)?;
            let (folder, reason) = match destination {
                Destination::Direct => (None, None),
                Destination::Folder(name) => (Some(name), None),
                Destination::Skip(reason) => (None, Some(reason)),
            };
            transaction
                .execute(
                    "UPDATE temp.file_operation_selection
                 SET destination_folder = ?2, skip_reason = ?3 WHERE ordinal = ?1",
                    params![ordinal, folder, reason],
                )
                .map_err(database_error)?;
            after = ordinal;
        }
    }
    transaction.commit().map_err(database_error)
}

fn resolve(
    connection: &Connection,
    selected: &OrganizationSelection,
    rule: Rule,
) -> Result<Destination, ScanError> {
    let (name, missing) = match rule {
        Rule::Direct => return Ok(Destination::Direct),
        Rule::Album => {
            let inspection =
                match organization::inspection::load(connection, std::slice::from_ref(selected)) {
                    Ok(value) => value,
                    Err(error) if error.code == "organization_evidence_pending" => {
                        return Ok(Destination::Skip("相册归属正在更新，尚不能确定目标".into()));
                    }
                    Err(error) => return Err(error),
                };
            let Some(decision) = inspection.decisions.first() else {
                return Ok(Destination::Skip("没有可用的相册归属".into()));
            };
            if inspection.decisions.len() != 1 || decision.has_conflict {
                return Ok(Destination::Skip(
                    "存在多个相册目标，未选择其中任何一个".into(),
                ));
            }
            let name = if let Some(album) = &decision.album {
                connection
                    .query_row(
                        "SELECT name FROM organization_albums AS album WHERE album_id = ?1
                     AND NOT EXISTS(SELECT 1 FROM organization_album_retirements AS retired
                       WHERE retired.album_id = album.album_id)",
                        [&album.album_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(database_error)?
            } else {
                None
            };
            (name, "没有可用的相册归属")
        }
        Rule::Category => {
            let projection = gallery::classification::effective_projection(
                "locations",
                gallery::classification::PredictionReadStrategy::Indexed,
            );
            let name = connection.query_row(
                &format!("SELECT ({projection}
                   SELECT category.label FROM effective
                   JOIN classification_categories AS category ON category.category_id = effective.category_id)
                 FROM asset_locations AS locations
                 JOIN library_roots AS roots ON roots.active_scan_id = locations.scan_id
                 WHERE locations.location_id = ?1 AND locations.asset_id = ?2 AND locations.source_generation = ?3"),
                params![selected.location_id, selected.asset_id, selected.source_generation as i64],
                |row| row.get::<_, Option<String>>(0),
            )
                .optional().map_err(database_error)?.flatten();
            (name, "没有可用的图片分类")
        }
        Rule::Date => {
            let name = connection.query_row(
                "SELECT substr(COALESCE(locations.capture_local_time, locations.file_local_time), 1, 7)
                 FROM asset_locations AS locations
                 JOIN library_roots AS roots ON roots.active_scan_id = locations.scan_id
                 WHERE locations.location_id = ?1 AND locations.asset_id = ?2 AND locations.source_generation = ?3",
                params![selected.location_id, selected.asset_id, selected.source_generation as i64],
                |row| row.get::<_, Option<String>>(0),
            )
                .optional().map_err(database_error)?.flatten();
            (name, "没有可用的图库日期")
        }
        Rule::Source => {
            let path = connection.query_row(
                "SELECT roots.path FROM library_roots AS roots
                 JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
                 WHERE locations.location_id = ?1 AND locations.asset_id = ?2 AND locations.source_generation = ?3",
                params![selected.location_id, selected.asset_id, selected.source_generation as i64],
                |row| row.get::<_, String>(0),
            )
                .optional().map_err(database_error)?;
            let name = path
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            (name, "来源文件夹没有可用的名称")
        }
    };
    Ok(match name.filter(|name| !name.is_empty()) {
        Some(name) => Destination::Folder(name),
        None => Destination::Skip(missing.into()),
    })
}
