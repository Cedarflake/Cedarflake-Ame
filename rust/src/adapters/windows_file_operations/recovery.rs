use crate::adapters::{exact_content_reader, revalidate_open_preview_source};
use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, MAX_FINGERPRINT_SOURCE_BYTES};
use crate::domain::file_operations::{
    FileOperationContentProof, FileOperationIssue, FileOperationIssueKind as Kind,
    FileOperationSource,
};
use crate::ports::file_operations::FileOperationControl;

use super::scope;

pub(super) fn verify_content(
    source: &FileOperationSource,
    proof: &FileOperationContentProof,
    control: &FileOperationControl,
) -> Result<(), FileOperationIssue> {
    if proof.scheme != EXACT_FINGERPRINT_SCHEME
        || proof.byte_length != source.state.file_size
        || !(1..=MAX_FINGERPRINT_SOURCE_BYTES).contains(&proof.byte_length)
    {
        return Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "Recovery requires supported content evidence for the authorized file",
        ));
    }
    if control.is_cancelled() {
        return Err(super::cancelled());
    }
    let (guard, name) = scope::open_source(source)?;
    let file = guard
        .open_operation_source(&name, &source.state, false)
        .map_err(|error| FileOperationIssue::new(Kind::DestinationChanged, error.to_string()))?;
    let fingerprint =
        exact_content_reader::hash_reader(&file, proof.byte_length, control.cancellation_flag())
            .map_err(|error| FileOperationIssue::new(Kind::OutcomeUnknown, error.to_string()))?
            .ok_or_else(super::cancelled)?;
    if fingerprint.digest != proof.digest {
        return Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "The destination content differs from the authorized file",
        ));
    }
    revalidate_open_preview_source(&file, &source.state)
        .map(|_| ())
        .map_err(|error| FileOperationIssue::new(Kind::DestinationChanged, error.message))
}
