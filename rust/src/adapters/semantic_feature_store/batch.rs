use std::collections::HashMap;

use rusqlite::{params_from_iter, types::Value};

use super::*;
use crate::domain::semantic_results::SEMANTIC_CANDIDATE_WINDOW;

pub(super) fn load(
    store: &SqliteSemanticFeatureStore,
    model: ModelCompatibility,
    contents: &[ImageContentIdentity],
    generation: u64,
) -> Result<Vec<Option<ImageFeature>>, ScanError> {
    if contents.len() > SEMANTIC_CANDIDATE_WINDOW || generation > i64::MAX as u64 {
        return Err(invalid("The feature read window is too large"));
    }
    if contents.is_empty() {
        return Ok(Vec::new());
    }
    let keys: Vec<_> = contents
        .iter()
        .map(|content| content.feature_key(model))
        .collect();
    let placeholders = vec!["?"; keys.len()].join(",");
    let mut parameters = vec![
        Value::Integer(model.dimensions() as i64 * 4 + 1),
        Value::Integer(generation as i64),
    ];
    parameters.extend(keys.iter().map(|key| Value::Blob(key.bytes().to_vec())));
    let mut statement = store
        .connection
        .prepare(&format!(
            "SELECT feature_key, substr(vector, 1, ?), substr(checksum, 1, 33)
         FROM image_features WHERE sequence <= ? AND feature_key IN ({placeholders})"
        ))
        .map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(parameters))
        .map_err(database_error)?;
    let mut found = HashMap::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let key: Vec<u8> = row.get(0).map_err(database_error)?;
        let key = AnalysisRevision::from_bytes(
            key.try_into().map_err(|_| invalid("Invalid feature key"))?,
        );
        let payload: Vec<u8> = row.get(1).map_err(database_error)?;
        let checksum: Vec<u8> = row.get(2).map_err(database_error)?;
        found.insert(key, decode(model, key, &payload, &checksum)?);
    }
    Ok(keys.iter().map(|key| found.get(key).cloned()).collect())
}
