use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FileOperationCatalogBinding, FileOperationCheckpoint,
    FileOperationPhase, invalid,
};
use crate::domain::file_operations::{
    FileAction, FileOperationIssue, FileOperationReceipt, FileOperationRecovery,
    FileOperationRequest,
};

mod proof;
use proof::{Destination, Source};

const MAX_RECORD_BYTES: usize = 512 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authorization {
    batch_id: String,
    ordinal: u64,
    authorized_unix_ms: i64,
    catalog: FileOperationCatalogBinding,
    action: FileAction,
    source: Source,
    destination: Option<Destination>,
}

pub(super) fn encode_authorization(value: &AuthorizedFileOperation) -> Result<String, ScanError> {
    value.validate()?;
    encode(&Authorization {
        batch_id: value.batch_id.clone(),
        ordinal: value.ordinal,
        authorized_unix_ms: value.authorized_unix_ms,
        catalog: value.catalog.clone(),
        action: value.request.action,
        source: (&value.request.source).into(),
        destination: value.request.destination.as_ref().map(Into::into),
    })
}

pub(super) fn decode_authorization(text: &str) -> Result<AuthorizedFileOperation, ScanError> {
    let value: Authorization = decode(text)?;
    let result = AuthorizedFileOperation {
        batch_id: value.batch_id,
        ordinal: value.ordinal,
        authorized_unix_ms: value.authorized_unix_ms,
        catalog: value.catalog,
        request: FileOperationRequest {
            action: value.action,
            source: value.source.into(),
            destination: value.destination.map(Into::into),
        },
    };
    result.validate()?;
    Ok(result)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    destination: Option<Source>,
    recovery: FileOperationRecovery,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    version: u64,
    phase: FileOperationPhase,
    destination: Option<Destination>,
    copied: Option<Source>,
    receipt: Option<Receipt>,
    issue: Option<FileOperationIssue>,
    catalog_revision: Option<u64>,
}

pub(super) fn encode_checkpoint(value: &FileOperationCheckpoint) -> Result<String, ScanError> {
    encode(&Checkpoint {
        version: value.version,
        phase: value.phase,
        destination: value.destination.as_ref().map(Into::into),
        copied: value.copied.as_ref().map(Into::into),
        receipt: value.receipt.as_ref().map(|receipt| Receipt {
            destination: receipt.destination.as_ref().map(Into::into),
            recovery: receipt.recovery,
        }),
        issue: value.issue.clone(),
        catalog_revision: value.catalog_revision,
    })
}

pub(super) fn decode_checkpoint(text: &str) -> Result<FileOperationCheckpoint, ScanError> {
    let value: Checkpoint = decode(text)?;
    Ok(FileOperationCheckpoint {
        version: value.version,
        phase: value.phase,
        destination: value.destination.map(Into::into),
        copied: value.copied.map(Into::into),
        receipt: value.receipt.map(|receipt| FileOperationReceipt {
            destination: receipt.destination.map(Into::into),
            recovery: receipt.recovery,
        }),
        issue: value.issue,
        catalog_revision: value.catalog_revision,
    })
}

pub(super) fn phase_name(phase: FileOperationPhase) -> &'static str {
    match phase {
        FileOperationPhase::Prepared => "prepared",
        FileOperationPhase::Executing => "executing",
        FileOperationPhase::NameOccupied => "name_occupied",
        FileOperationPhase::Copied => "copied",
        FileOperationPhase::RemovingSource => "removing_source",
        FileOperationPhase::NativeCompleted => "native_completed",
        FileOperationPhase::Published => "published",
        FileOperationPhase::Failed => "failed",
        FileOperationPhase::Cancelled => "cancelled",
        FileOperationPhase::Uncertain => "uncertain",
    }
}

fn encode(value: &impl Serialize) -> Result<String, ScanError> {
    let text = serde_json::to_string(value).map_err(|error| invalid(error.to_string()))?;
    if text.len() > MAX_RECORD_BYTES {
        return Err(invalid("The operation record exceeds its storage bound"));
    }
    Ok(text)
}

fn decode<T: DeserializeOwned>(text: &str) -> Result<T, ScanError> {
    if text.len() > MAX_RECORD_BYTES {
        return Err(invalid("The operation record exceeds its storage bound"));
    }
    serde_json::from_str(text).map_err(|error| invalid(error.to_string()))
}
