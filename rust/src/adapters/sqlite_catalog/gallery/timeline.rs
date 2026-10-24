use rusqlite::{Transaction, params_from_iter};

use crate::domain::{GalleryQuery, GallerySortKey, GalleryTimeBucket, GalleryTimeline, ScanError};

use super::super::{database_error, sqlite_unsigned};
use super::{
    BuiltGalleryQuery, gallery_direction_sql, gallery_order_expressions, gallery_source,
    push_gallery_filters, push_source_filters,
};

pub(crate) fn load_gallery_timeline(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    query_id: &str,
    revision: u64,
) -> Result<GalleryTimeline, ScanError> {
    let built = build_gallery_timeline_query(query);
    let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
    let rows = statement
        .query_map(params_from_iter(built.parameters.iter()), |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(database_error)?;
    let mut total_items = 0_u64;
    let mut buckets = Vec::new();
    for row in rows {
        let (month_key, item_count, aspect_ratio_milli_sum) = row.map_err(database_error)?;
        let item_count = sqlite_unsigned(item_count, "timeline bucket item count")?;
        let aspect_ratio_milli_sum =
            sqlite_unsigned(aspect_ratio_milli_sum, "timeline bucket aspect ratio sum")?;
        total_items = total_items.checked_add(item_count).ok_or_else(|| {
            ScanError::new(
                "catalog_timeline_count_invalid",
                "The gallery timeline item count exceeds the supported range",
            )
        })?;
        if query.organization_queue.is_none() && !matches!(query.sort_key, GallerySortKey::FileName)
        {
            buckets.push(GalleryTimeBucket {
                month_key,
                item_count,
                aspect_ratio_milli_sum,
            });
        }
    }
    drop(statement);
    let total_source_items = if !query.folds_duplicates() {
        total_items
    } else {
        load_source_count(transaction, query)?
    };
    Ok(GalleryTimeline {
        revision,
        query_id: query_id.to_owned(),
        total_items,
        total_source_items,
        buckets,
    })
}

fn load_source_count(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<u64, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_source_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let count: i64 = transaction
        .query_row(
            &format!(
                "SELECT COUNT(*)
             {source}
             {where_clause}",
                source = gallery_source(query),
            ),
            params_from_iter(parameters.iter()),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    sqlite_unsigned(count, "gallery source item count")
}

fn build_gallery_timeline_query(query: &GalleryQuery) -> BuiltGalleryQuery {
    let order = gallery_order_expressions(query);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let month = order.month.unwrap_or("NULL");
    let direction = gallery_direction_sql(query.order_direction());
    BuiltGalleryQuery {
        sql: format!(
            "SELECT {month} AS month_key,
                    COUNT(*),
                    SUM(
                      CASE
                        WHEN locations.width <= 0 OR locations.height <= 0 THEN 1000
                        WHEN locations.width * 5 < locations.height THEN 200
                        WHEN locations.width > locations.height * 5 THEN 5000
                        ELSE (locations.width * 1000) / locations.height
                      END
                    ) AS aspect_ratio_milli_sum
             {source}
             {where_clause}
             GROUP BY {month}
             ORDER BY (month_key IS NULL), month_key {direction}",
            source = gallery_source(query),
        ),
        parameters,
    }
}
