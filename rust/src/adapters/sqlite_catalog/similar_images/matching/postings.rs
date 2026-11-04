use super::*;

pub(super) const WINDOW_SIZE: usize = 128;

pub(super) struct Posting {
    pub evidence_id: i64,
    pub group_id: String,
    pub variant: i64,
    pub words: [u16; 4],
}

pub(super) fn variants(transaction: &Transaction<'_>, id: i64) -> Result<Vec<[u16; 4]>, ScanError> {
    transaction.prepare("SELECT word0, word1, word2, word3 FROM similarity_variants WHERE evidence_id = ?1 ORDER BY variant LIMIT 8").map_err(database_error)?
        .query_map([id], |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])).map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(database_error)
}

pub(super) fn load(
    transaction: &Transaction<'_>,
    work: &MatchWork,
    neighbors: &[u16],
) -> Result<Vec<Posting>, ScanError> {
    let word = format!("word{}", work.band);
    let after_word = *neighbors.get(work.neighbor).ok_or_else(invalid)?;
    let keys = serde_json::to_string(neighbors).map_err(|_| invalid())?;
    let references = work.phase == MatchPhase::References;
    let (table, identity) = if references {
        ("similarity_reference_variants", "group_id")
    } else {
        ("similarity_variants", "evidence_id")
    };
    let columns = if references {
        "0, group_id"
    } else {
        "evidence_id, ''"
    };
    let sql = format!(
        "SELECT {columns}, variant, word0, word1, word2, word3 FROM {table}
         WHERE {word} IN(SELECT value FROM json_each(?1)) AND ({word}, {identity}, variant) > (?2, ?3, ?4)
         ORDER BY {word}, {identity}, variant LIMIT ?5"
    );
    let after = if references {
        rusqlite::types::Value::Text(work.after_group_id.clone())
    } else {
        rusqlite::types::Value::Integer(work.after_evidence_id)
    };
    transaction
        .prepare(&sql)
        .map_err(database_error)?
        .query_map(
            params![
                keys,
                after_word,
                after,
                work.after_variant,
                WINDOW_SIZE as i64
            ],
            |row| {
                Ok(Posting {
                    evidence_id: row.get(0)?,
                    group_id: row.get(1)?,
                    variant: row.get(2)?,
                    words: [row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?],
                })
            },
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)
}
