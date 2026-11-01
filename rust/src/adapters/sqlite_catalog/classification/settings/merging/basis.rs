use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Deserialize;

use crate::domain::ScanError;
use crate::domain::classification::discovery::CategoryNamingPurpose;
use crate::domain::classification::maintenance::{CategoryMergeBasis, CategoryMergeSnapshot};

use super::super::super::{codec, invalid_record};
use super::super::retirement;
use super::database_error;

#[derive(Deserialize)]
struct NamingEvidence {
    kind: String,
    category: String,
    category_revision: String,
    label_pattern: String,
    comparison_pattern: String,
}

pub(super) fn snapshot(connection: &Connection) -> Result<CategoryMergeSnapshot, ScanError> {
    let categories = retirement::snapshot(connection)?;
    let mut bases = BTreeMap::new();
    let Some(taxonomy) = categories.configuration.catalog.taxonomy.as_ref() else {
        return Ok(CategoryMergeSnapshot { categories, bases });
    };
    let active: Vec<_> = taxonomy
        .categories()
        .iter()
        .map(|category| category.id().as_str())
        .collect();
    let encoded = codec::encode(&active, 32768)?;
    let mut statement = connection.prepare(
        "SELECT origin.category_id, evidence.evidence_revision,
                json_extract(evidence.payload, '$.version'),
                json_extract(evidence.payload, '$.naming'),
                json_extract(evidence.payload, '$.criteria.minimum_members'),
                EXISTS (SELECT 1 FROM json_each(evidence.payload, '$.candidates') AS candidate
                    WHERE json_extract(candidate.value, '$.category') = origin.category_id
                      AND json_extract(candidate.value, '$.concept') = origin.concept_id)
         FROM json_each(?1) AS active
         JOIN classification_managed_categories AS origin ON origin.category_id = active.value
         JOIN classification_maintenance_evidence AS evidence ON evidence.operation_id = origin.operation_id
         WHERE origin.purpose_category_id IS NOT NULL"
    ).map_err(database_error)?;
    let mut rows = statement.query([encoded]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let category_id = codec::category_id(row.get(0).map_err(database_error)?)?;
        let evidence = codec::read_revision(&row.get::<_, String>(1).map_err(database_error)?)?;
        let version: i64 = row.get(2).map_err(database_error)?;
        let naming: NamingEvidence =
            codec::decode(&row.get::<_, String>(3).map_err(database_error)?, 16384)?;
        let minimum: i64 = row.get(4).map_err(database_error)?;
        let matches_origin: bool = row.get(5).map_err(database_error)?;
        let origin = categories
            .origins
            .get(&category_id)
            .ok_or_else(invalid_record)?;
        if version != 1
            || !matches_origin
            || naming.kind != "purpose"
            || !(2..=511).contains(&minimum)
            || origin.purpose.as_ref().map(|category| category.as_str())
                != Some(naming.category.as_str())
        {
            return Err(invalid_record());
        }
        let parent_revision = codec::read_revision(&naming.category_revision)?;
        let Some(parent) = taxonomy.categories().iter().find(|category| {
            category.id().as_str() == naming.category
                && category.semantic_revision() == parent_revision
        }) else {
            continue;
        };
        let purpose =
            CategoryNamingPurpose::new(parent, naming.label_pattern, naming.comparison_pattern)
                .map_err(|_| invalid_record())?;
        bases.insert(
            category_id,
            CategoryMergeBasis {
                naming: purpose,
                minimum_members: minimum as usize,
                evidence,
            },
        );
    }
    Ok(CategoryMergeSnapshot { categories, bases })
}
