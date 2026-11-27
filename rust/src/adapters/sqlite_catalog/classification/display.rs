use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryId, ClassificationContent, ClassificationRecordId, ClassificationRun,
    ReviewRecommendation, Taxonomy,
};

use super::super::database_error;
use super::{codec, predictions};

pub(super) struct DisplayPrediction {
    pub prediction_id: ClassificationRecordId,
    pub category: CategoryId,
    pub review: ReviewRecommendation,
}

pub(super) fn for_content(
    connection: &Connection,
    content: &ClassificationContent,
) -> Result<Option<DisplayPrediction>, ScanError> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT evidence.payload, run.payload FROM classification_display_projection AS display
         JOIN classification_predictions AS evidence USING(publication_id)
         JOIN classification_runs AS run ON run.run_id = evidence.run_id
         WHERE display.fingerprint_scheme = ?1 AND display.digest = ?2 AND display.byte_length = ?3
         ORDER BY display.publication_id DESC LIMIT 1",
            params![
                content.scheme.as_str(),
                codec::digest(&content.digest),
                super::super::sqlite_integer(content.byte_length, "classification content length")?
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    row.map(|(payload, run)| {
        let evidence = codec::prediction::load(&payload, codec::decode_run(&run)?)?;
        Ok(DisplayPrediction {
            prediction_id: evidence.id().clone(),
            category: evidence.suggestion().leading().category_id.clone(),
            review: evidence.review_recommendation(),
        })
    })
    .transpose()
}

pub(super) struct RetainedDisplay {
    generation: i64,
    run_id: String,
}

impl RetainedDisplay {
    pub(super) fn capture(
        connection: &Connection,
        replacement: &ClassificationRun,
    ) -> Result<Option<Self>, ScanError> {
        let retained: Option<(i64, String)> = connection.query_row(
            "SELECT display.generation, display.run_id FROM classification_display_state AS display
             JOIN classification_state AS state ON state.singleton = display.singleton
               AND state.active_rules = display.rules_revision
               AND state.configuration_revision IS display.configuration_revision
             WHERE display.run_id IS NOT NULL", [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(database_error)?;
        let Some((generation, run_id)) = retained else {
            return Ok(None);
        };
        if predictions::load_run(connection, &run_id)?.model() != replacement.model() {
            return Ok(None);
        }
        Ok(Some(Self { generation, run_id }))
    }

    pub(super) fn retain(self, connection: &Connection) -> Result<(), ScanError> {
        connection
            .execute(
                "UPDATE classification_display_state SET generation = ?1, run_id = ?2,
               rules_revision = (SELECT active_rules FROM classification_state),
               configuration_revision = (SELECT configuration_revision FROM classification_state)
             WHERE singleton = 1",
                params![self.generation, self.run_id],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

pub(super) fn begin(connection: &Connection, run: &ClassificationRun) -> Result<(), ScanError> {
    connection
        .execute(
            "UPDATE classification_display_state SET generation = generation + 1,
           run_id = ?1, rules_revision = ?2, configuration_revision = ?3
         WHERE singleton = 1 AND (configuration_revision IS NOT ?3 OR rules_revision IS NOT ?2)",
            params![
                run.id().as_str(),
                codec::revision(run.primary_rules()),
                codec::revision(run.configuration())
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn publish(connection: &Connection, publication: i64) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO classification_display_predictions
           (fingerprint_scheme, digest, byte_length, publication_id, generation, category_id, review)
         SELECT prediction.fingerprint_scheme, prediction.digest, prediction.byte_length,
           prediction.publication_id, display.generation, prediction.category_id, prediction.review
         FROM classification_prediction_projection AS prediction
         CROSS JOIN classification_display_state AS display
         WHERE prediction.publication_id = ?1
         ON CONFLICT(fingerprint_scheme, digest, byte_length) DO UPDATE SET
           publication_id = excluded.publication_id, generation = excluded.generation,
           category_id = excluded.category_id, review = excluded.review", [publication],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn retire_categories(
    connection: &Connection,
    previous: Option<&Taxonomy>,
    replacement: &Taxonomy,
) -> Result<(), ScanError> {
    for category in previous.into_iter().flat_map(Taxonomy::categories) {
        if replacement
            .categories()
            .iter()
            .any(|current| current.id() == category.id())
        {
            continue;
        }
        connection.execute(
            "INSERT INTO classification_display_category_floors(category_id, first_publication_id)
             VALUES (?1, (SELECT COALESCE(MAX(publication_id), 0) + 1 FROM classification_predictions))
             ON CONFLICT(category_id) DO UPDATE SET first_publication_id = excluded.first_publication_id",
            [category.id().as_str()],
        ).map_err(database_error)?;
    }
    Ok(())
}
