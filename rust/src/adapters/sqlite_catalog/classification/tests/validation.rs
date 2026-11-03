use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

fn validation_steps(catalog: &SqliteCatalog) -> u64 {
    let count = Arc::new(AtomicU64::new(0));
    let observed = Arc::clone(&count);
    catalog
        .connection
        .progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    super::super::validation::catalog(&catalog.connection).unwrap();
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    count.load(Ordering::Relaxed)
}

#[test]
fn prediction_history_validation_bounds_repeated_taxonomy_work() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let baseline = validation_steps(&catalog);
    for index in 0..64 {
        predict(
            &mut catalog,
            &run,
            "a",
            &format!("history-{index}"),
            "drawing",
        );
    }
    let grown = validation_steps(&catalog);
    assert!(
        grown - baseline < 64 * 60,
        "baseline={baseline}, grown={grown}"
    );
}

#[test]
fn reused_run_context_preserves_every_prediction_check() {
    for mutation in [
        "UPDATE classification_predictions SET payload = json_set(payload, '$.id', 'wrong') WHERE prediction_id = 'second'",
        "UPDATE classification_predictions SET digest = replace(digest, 'a', 'b') WHERE prediction_id = 'second'",
        "UPDATE classification_predictions SET payload = json_remove(payload, '$.initial.categories[1]') WHERE prediction_id = 'second'",
    ] {
        let (_directory, mut catalog) = fixture();
        publish(&mut catalog, "a", 'a');
        let run = setup(&mut catalog);
        predict(&mut catalog, &run, "a", "first", "drawing");
        predict(&mut catalog, &run, "a", "second", "drawing");
        catalog.connection.execute(mutation, []).unwrap();
        assert_eq!(
            super::super::validation::catalog(&catalog.connection)
                .unwrap_err()
                .code,
            "classification_record_invalid",
            "{mutation}",
        );
    }
}

#[test]
fn validation_handles_interleaved_runs_without_cross_run_reuse() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let first = setup(&mut catalog);
    predict(&mut catalog, &first, "a", "first-prediction", "drawing");
    predict(&mut catalog, &first, "a", "third-prediction", "drawing");
    let second =
        ClassificationRun::new(id("second-run"), first.model(), first.primary_rules(), None)
            .unwrap();
    catalog.begin_classification_run(&second, None).unwrap();
    predict(&mut catalog, &second, "a", "second-prediction", "photo");
    catalog.connection.execute(
        "UPDATE classification_predictions SET publication_id = 100 WHERE prediction_id = 'third-prediction'",
        [],
    ).unwrap();
    super::super::validation::catalog(&catalog.connection).unwrap();
}
