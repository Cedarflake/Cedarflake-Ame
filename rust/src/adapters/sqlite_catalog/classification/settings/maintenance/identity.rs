use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::CategoryId;
use crate::domain::classification::maintenance::{
    CategoryActivation, CategoryExpansionPlan, CategoryExpansionSnapshot,
};

use super::{changed, database_error, maintenance};

pub(super) fn snapshot(
    connection: &Connection,
    proposed: &[CategoryId],
) -> Result<CategoryExpansionSnapshot, ScanError> {
    if proposed.is_empty() || proposed.len() > 64 {
        return Err(ScanError::new(
            "classification_category_limit",
            "Category expansion exceeds its candidate window",
        ));
    }
    let configuration = super::super::load(connection)?;
    let model = super::super::authority::active_model(connection, &configuration)?;
    let encoded =
        serde_json::to_string(&proposed.iter().map(CategoryId::as_str).collect::<Vec<_>>())
            .map_err(|_| changed())?;
    let mut statement = connection
        .prepare(
            "SELECT category_id, label FROM classification_categories
         WHERE category_id IN (SELECT value FROM json_each(?1))",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([encoded], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    let mut known_categories = BTreeMap::new();
    for row in rows {
        let (id, label) = row.map_err(database_error)?;
        known_categories.insert(CategoryId::new(id).map_err(|_| changed())?, label);
    }
    let mut origin_categories: BTreeSet<_> = proposed.iter().cloned().collect();
    if let Some(rules) = &configuration.catalog.taxonomy {
        origin_categories.extend(
            rules
                .categories()
                .iter()
                .map(|category| category.id().clone()),
        );
    }
    let managed_categories = maintenance::load(
        connection,
        &origin_categories.into_iter().collect::<Vec<_>>(),
    )?;
    Ok(CategoryExpansionSnapshot {
        configuration,
        model,
        known_categories,
        managed_categories,
    })
}

pub(super) fn validate(
    snapshot: &CategoryExpansionSnapshot,
    plan: &CategoryExpansionPlan,
) -> Result<(), ScanError> {
    let mut expected_known = BTreeSet::new();
    for candidate in &plan.provenance.candidates {
        match &candidate.activation {
            CategoryActivation::New => {
                if snapshot
                    .known_categories
                    .contains_key(&candidate.category_id)
                {
                    return Err(changed());
                }
            }
            CategoryActivation::Reactivate(origin) => {
                let category = plan
                    .configuration
                    .setup
                    .primary()
                    .categories()
                    .iter()
                    .find(|category| category.id() == &candidate.category_id)
                    .ok_or_else(changed)?;
                if snapshot.reactivation_origin(category) != Some(origin) {
                    return Err(changed());
                }
                expected_known.insert(candidate.category_id.clone());
            }
        }
    }
    if snapshot
        .known_categories
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != expected_known
    {
        return Err(changed());
    }
    Ok(())
}
