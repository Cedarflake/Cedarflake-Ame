use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, Row, params};

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, ConceptId};
use crate::domain::classification::maintenance::{CategoryExpansionPlan, ManagedCategoryOrigin};
use crate::domain::classification::{CategoryId, Taxonomy};

use super::super::database_error;
use super::{codec, invalid_record};

mod evidence;
pub(super) use evidence::encode;

pub(super) fn publish(
    connection: &Connection,
    plan: &CategoryExpansionPlan,
    payload: &str,
) -> Result<(), ScanError> {
    let operation = plan.configuration.operation_id.as_str();
    connection.execute(
        "INSERT INTO classification_maintenance_evidence(operation_id, run_id, evidence_revision, payload)
         VALUES (?1, ?2, ?3, ?4)",
        params![operation, plan.configuration.setup.run().id().as_str(), codec::revision(plan.evidence), payload],
    ).map_err(database_error)?;
    let purpose = match &plan.provenance.naming {
        CategoryNaming::Unconditioned => None,
        CategoryNaming::WithinCategory(purpose) => Some(purpose.category().as_str()),
    };
    for candidate in &plan.provenance.candidates {
        let category = plan
            .configuration
            .setup
            .primary()
            .categories()
            .iter()
            .find(|category| category.id() == &candidate.category_id)
            .ok_or_else(invalid_record)?;
        connection.execute(
            "INSERT INTO classification_managed_categories(category_id, concept_id, purpose_category_id,
                 operation_id, admitted_label, admitted_semantic_revision, protected)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![category.id().as_str(), codec::digest(candidate.concept_id.bytes()), purpose,
                operation, category.label(), codec::revision(category.semantic_revision())],
        ).map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn load(
    connection: &Connection,
    categories: &[CategoryId],
) -> Result<BTreeMap<CategoryId, ManagedCategoryOrigin>, ScanError> {
    let encoded = codec::encode(
        &categories
            .iter()
            .map(CategoryId::as_str)
            .collect::<Vec<_>>(),
        32768,
    )?;
    let mut statement = connection.prepare(
        "SELECT category_id, concept_id, purpose_category_id, operation_id, admitted_label,
                admitted_semantic_revision, protected
         FROM classification_managed_categories WHERE category_id IN (SELECT value FROM json_each(?1))"
    ).map_err(database_error)?;
    let mut rows = statement.query([encoded]).map_err(database_error)?;
    let mut origins = BTreeMap::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let origin = read_origin(row)?;
        origins.insert(origin.category_id.clone(), origin);
    }
    Ok(origins)
}

fn read_origin(row: &Row<'_>) -> Result<ManagedCategoryOrigin, ScanError> {
    let purpose: Option<String> = row.get(2).map_err(database_error)?;
    let origin = ManagedCategoryOrigin {
        category_id: codec::category_id(row.get(0).map_err(database_error)?)?,
        concept_id: ConceptId::from_bytes(codec::read_digest(
            &row.get::<_, String>(1).map_err(database_error)?,
        )?),
        purpose: purpose.map(codec::category_id).transpose()?,
        operation_id: codec::identifier(row.get(3).map_err(database_error)?)?,
        admitted_label: row.get(4).map_err(database_error)?,
        admitted_semantic_revision: codec::read_revision(
            &row.get::<_, String>(5).map_err(database_error)?,
        )?,
        protected: row.get(6).map_err(database_error)?,
    };
    origin.validate().map_err(|_| invalid_record())?;
    Ok(origin)
}

pub(in crate::adapters::sqlite_catalog) fn validate_origins(
    connection: &Connection,
) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT category_id, concept_id, purpose_category_id, operation_id, admitted_label,
                admitted_semantic_revision, protected FROM classification_managed_categories",
        )
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        read_origin(row)?;
    }
    Ok(())
}

pub(super) fn protect_manual_changes(
    connection: &Connection,
    previous: Option<&Taxonomy>,
    current: &Taxonomy,
) -> Result<(), ScanError> {
    let previous: BTreeMap<_, _> = previous
        .into_iter()
        .flat_map(Taxonomy::categories)
        .map(|category| (category.id(), category))
        .collect();
    let current: BTreeMap<_, _> = current
        .categories()
        .iter()
        .map(|category| (category.id(), category))
        .collect();
    let identities: BTreeSet<_> = previous.keys().chain(current.keys()).copied().collect();
    let changed: Vec<_> = identities
        .into_iter()
        .filter(|id| previous.get(id) != current.get(id))
        .map(CategoryId::as_str)
        .collect();
    if changed.is_empty() {
        return Ok(());
    }
    let encoded = codec::encode(&changed, 65536)?;
    connection
        .execute(
            "UPDATE classification_managed_categories SET protected = 1
         WHERE protected = 0 AND category_id IN (SELECT value FROM json_each(?1))",
            [encoded],
        )
        .map_err(database_error)?;
    Ok(())
}
