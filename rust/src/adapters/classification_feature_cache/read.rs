use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use super::{
    ClassificationRevision, Embedding, ScanError, database_error, invalid, payload_digest,
};

pub(super) fn load_features(
    connection: &mut Connection,
    keys: &[ClassificationRevision],
    dimensions: usize,
) -> Result<Vec<Option<Embedding>>, ScanError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let mut features = Vec::with_capacity(keys.len());
    for key in keys {
        let feature = read_validated(&transaction, *key, dimensions)?;
        if feature.is_some() {
            super::storage::touch(&transaction, *key)?;
        }
        features.push(feature);
    }
    transaction.commit().map_err(database_error)?;
    Ok(features)
}

fn read_validated(
    connection: &Connection,
    key: ClassificationRevision,
    dimensions: usize,
) -> Result<Option<Embedding>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT length(payload), substr(payload, 1, 16384), substr(checksum, 1, 33)
             FROM features WHERE key = ?1",
            [key.bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((length, bytes, checksum)) = stored else {
        return Ok(None);
    };
    if length != (dimensions * 4) as i64
        || bytes.len() as i64 != length
        || checksum.as_slice() != payload_digest(key, &bytes).bytes()
    {
        return Err(invalid("The stored feature failed its integrity check"));
    }
    let values = bytes
        .chunks_exact(4)
        .map(|value| f32::from_le_bytes([value[0], value[1], value[2], value[3]]))
        .collect();
    Embedding::from_normalized_values(values)
        .map(Some)
        .map_err(|error| invalid(error.to_string()))
}
