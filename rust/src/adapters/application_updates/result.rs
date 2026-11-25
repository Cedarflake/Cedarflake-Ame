use std::fs;
use std::io::Read;

use serde::Deserialize;

use super::package::{cache_directory, reject_reparse, storage};
use crate::domain::application_update::{
    ApplicationUpdateError, ApplicationUpdateOutcome, ApplicationUpdateResult,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u32,
    version: String,
    code: u32,
}

pub(super) fn take_result(
    current: &str,
) -> Result<Option<ApplicationUpdateResult>, ApplicationUpdateError> {
    let path = cache_directory()?.join("result.json");
    take_result_at(&path, current)
}

fn take_result_at(
    path: &std::path::Path,
    current: &str,
) -> Result<Option<ApplicationUpdateResult>, ApplicationUpdateError> {
    reject_reparse(&path)?;
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage(error)),
    };
    let mut bytes = Vec::new();
    file.take(1025).read_to_end(&mut bytes).map_err(storage)?;
    let receipt = serde_json::from_slice::<Receipt>(&bytes)
        .ok()
        .filter(|receipt| bytes.len() <= 1024 && receipt.schema == 1);
    fs::remove_file(path).map_err(storage)?;
    let Some(receipt) = receipt else {
        return Ok(Some(ApplicationUpdateResult {
            outcome: ApplicationUpdateOutcome::Failed,
            code: 13,
        }));
    };
    let outcome = match receipt.code {
        0 | 3010 if receipt.version == current => ApplicationUpdateOutcome::Updated,
        1223 | 1602 => ApplicationUpdateOutcome::Cancelled,
        _ => ApplicationUpdateOutcome::Failed,
    };
    Ok(Some(ApplicationUpdateResult {
        outcome,
        code: if receipt.code == 0 && outcome == ApplicationUpdateOutcome::Failed {
            1603
        } else {
            receipt.code
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipts_are_consumed_once_and_corruption_does_not_disable_future_checks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("result.json");
        for (contents, expected, code) in [
            (
                r#"{"schema":1,"version":"1.2.3","code":0}"#,
                ApplicationUpdateOutcome::Updated,
                0,
            ),
            (
                r#"{"schema":1,"version":"1.2.4","code":0}"#,
                ApplicationUpdateOutcome::Failed,
                1603,
            ),
            (
                r#"{"schema":1,"version":"1.2.4","code":1223}"#,
                ApplicationUpdateOutcome::Cancelled,
                1223,
            ),
            ("interrupted receipt", ApplicationUpdateOutcome::Failed, 13),
        ] {
            fs::write(&path, contents).unwrap();
            let result = take_result_at(&path, "1.2.3").unwrap().unwrap();
            assert_eq!(result.outcome, expected);
            assert_eq!(result.code, code);
            assert!(take_result_at(&path, "1.2.3").unwrap().is_none());
        }
    }
}
