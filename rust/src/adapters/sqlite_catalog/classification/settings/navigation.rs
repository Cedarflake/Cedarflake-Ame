use std::collections::BTreeSet;

use crate::adapters::sqlite_catalog::gallery::classification::{
    PredictionReadStrategy, effective_projection,
};
use crate::domain::ScanError;
use crate::domain::classification::CategoryId;
use crate::domain::classification_configuration::ClassificationNavigationSnapshot;
use crate::ports::classification_configuration::ClassificationNavigationRepository;

use super::{SqliteCatalog, codec, database_error};

impl ClassificationNavigationRepository for SqliteCatalog {
    fn load_classification_navigation(
        &self,
    ) -> Result<ClassificationNavigationSnapshot, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let configuration = super::load(&transaction)?;
        let active: BTreeSet<_> = configuration
            .catalog
            .taxonomy
            .as_ref()
            .into_iter()
            .flat_map(|rules| rules.categories())
            .map(|category| category.id().clone())
            .collect();
        let mut populated_categories = BTreeSet::new();
        let has_evidence: bool = transaction.query_row(
            "SELECT EXISTS (SELECT 1 FROM classification_decisions WHERE category_id IS NOT NULL)
                 OR EXISTS (SELECT 1 FROM classification_prediction_projection)",
            [], |row| row.get(0),
        ).map_err(database_error)?;
        if !active.is_empty() && has_evidence {
            let active_ids = codec::encode(
                &active.iter().map(CategoryId::as_str).collect::<Vec<_>>(),
                64 * 1024,
            )?;
            let query = format!(
                "SELECT DISTINCT category_id FROM (
                    SELECT ({} SELECT category_id FROM effective) AS category_id
                    FROM library_roots AS roots
                    CROSS JOIN asset_locations AS locations
                      ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id
                 ) WHERE category_id IN (SELECT value FROM json_each(?1)) LIMIT ?2",
                effective_projection("locations", PredictionReadStrategy::Shared),
            );
            let mut statement = transaction.prepare(&query).map_err(database_error)?;
            let rows = statement
                .query_map(rusqlite::params![active_ids, active.len() as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(database_error)?;
            for row in rows {
                let category: CategoryId = codec::category_id(row.map_err(database_error)?)?;
                populated_categories.insert(category);
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(ClassificationNavigationSnapshot {
            configuration,
            populated_categories,
        })
    }
}
