use std::path::Path;

use super::naming::is_numbered_name;
use super::{
    AuthorizedFileOperation, FileOperationCheckpoint, FileOperationEvent as Event,
    FileOperationPhase as Phase, invalid, validate_source,
};
use crate::domain::ScanError;
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationIssueKind, FileOperationReceipt,
    FileOperationRecovery, FileOperationSource,
};

impl FileOperationCheckpoint {
    pub fn advance(
        &self,
        authorization: &AuthorizedFileOperation,
        event: Event,
    ) -> Result<Self, ScanError> {
        authorization.validate()?;
        let action = authorization.request.action;
        let mut next = self.clone();
        next.version = self
            .version
            .checked_add(1)
            .ok_or_else(|| invalid("Operation version exhausted"))?;
        next.issue = None;
        next.phase = match event {
            Event::Started(destination)
                if matches!(self.phase, Phase::Prepared | Phase::NameOccupied) =>
            {
                validate_destination(
                    authorization.request.destination.as_ref(),
                    destination.as_ref(),
                )?;
                next.destination = destination;
                Phase::Executing
            }
            Event::NameOccupied if self.phase == Phase::Executing && self.destination.is_some() => {
                Phase::NameOccupied
            }
            Event::Copied(destination)
                if action == FileAction::Move
                    && (self.phase == Phase::Executing
                        || matches!(self.phase, Phase::RemovingSource | Phase::Uncertain)
                            && self.copied.as_ref().is_some_and(|previous| {
                                previous.state.file_identity == destination.state.file_identity
                            })) =>
            {
                validate_completed_destination(self.destination.as_ref(), &destination)?;
                if destination.state.file_size != authorization.request.source.state.file_size {
                    return Err(invalid("A copied move destination has the wrong size"));
                }
                next.copied = Some(destination);
                Phase::Copied
            }
            Event::RemovingSource
                if matches!(self.phase, Phase::Copied | Phase::Failed | Phase::Cancelled)
                    && action == FileAction::Move
                    && self.copied.is_some() =>
            {
                Phase::RemovingSource
            }
            Event::NativeCompleted(receipt)
                if matches!(
                    self.phase,
                    Phase::Executing | Phase::RemovingSource | Phase::Uncertain
                ) =>
            {
                validate_receipt(authorization, self.destination.as_ref(), &receipt)?;
                if self.copied.is_none()
                    && matches!(action, FileAction::Move | FileAction::Rename)
                    && receipt.destination.as_ref().is_none_or(|destination| {
                        destination.state.file_identity
                            != authorization.request.source.state.file_identity
                    })
                {
                    return Err(invalid(
                        "A same-volume move must retain the source identity",
                    ));
                }
                if let Some(copied) = &self.copied {
                    let destination = receipt
                        .destination
                        .as_ref()
                        .ok_or_else(|| invalid("The move receipt lost its copied destination"))?;
                    if destination.state.file_identity != copied.state.file_identity {
                        return Err(invalid("The move receipt changed the copied file identity"));
                    }
                }
                next.receipt = Some(receipt);
                Phase::NativeCompleted
            }
            Event::Published(revision)
                if self.phase == Phase::NativeCompleted && self.receipt.is_some() =>
            {
                next.catalog_revision = Some(revision);
                Phase::Published
            }
            Event::Failed(issue)
                if issue.kind != FileOperationIssueKind::OutcomeUnknown
                    && matches!(
                        self.phase,
                        Phase::Prepared
                            | Phase::Executing
                            | Phase::NameOccupied
                            | Phase::Copied
                            | Phase::RemovingSource
                    ) =>
            {
                next.issue = Some(issue);
                Phase::Failed
            }
            Event::Cancelled
                if matches!(
                    self.phase,
                    Phase::Prepared
                        | Phase::Executing
                        | Phase::NameOccupied
                        | Phase::Copied
                        | Phase::RemovingSource
                ) =>
            {
                Phase::Cancelled
            }
            Event::Uncertain(issue)
                if matches!(
                    self.phase,
                    Phase::Executing
                        | Phase::RemovingSource
                        | Phase::NativeCompleted
                        | Phase::Copied
                        | Phase::Failed
                        | Phase::Cancelled
                ) =>
            {
                next.issue = Some(issue);
                Phase::Uncertain
            }
            _ => return Err(invalid("The event cannot advance this operation state")),
        };
        Ok(next)
    }
}

fn validate_destination(
    authorized: Option<&FileOperationDestination>,
    actual: Option<&FileOperationDestination>,
) -> Result<(), ScanError> {
    match (authorized, actual) {
        (None, None) => Ok(()),
        (Some(expected), Some(actual))
            if expected.directory.path == actual.directory.path
                && expected.directory.identity == actual.directory.identity
                && is_numbered_name(&expected.name, &actual.name) =>
        {
            Ok(())
        }
        _ => Err(invalid(
            "Execution changed the authorized destination policy",
        )),
    }
}

fn validate_completed_destination(
    expected: Option<&FileOperationDestination>,
    actual: &FileOperationSource,
) -> Result<(), ScanError> {
    validate_source(actual)?;
    let expected = expected.ok_or_else(|| invalid("The operation has no destination"))?;
    if expected.directory.path != actual.parent.path
        || expected.directory.identity != actual.parent.identity
        || Path::new(&actual.state.absolute_path)
            != Path::new(&expected.directory.path).join(&expected.name)
    {
        return Err(invalid("The native result changed the destination"));
    }
    Ok(())
}

fn validate_receipt(
    authorization: &AuthorizedFileOperation,
    expected: Option<&FileOperationDestination>,
    receipt: &FileOperationReceipt,
) -> Result<(), ScanError> {
    match (expected, receipt.destination.as_ref()) {
        (Some(expected), Some(actual)) => {
            validate_completed_destination(Some(expected), actual)?;
            if actual.state.file_size != authorization.request.source.state.file_size {
                return Err(invalid("The transfer receipt changed the file size"));
            }
        }
        (None, None) => {}
        _ => return Err(invalid("The receipt does not match the transfer action")),
    }
    if receipt.recovery != FileOperationRecovery::None
        && authorization.request.action != FileAction::Recycle
    {
        return Err(invalid(
            "Only a recycled file has a Recycle Bin recovery route",
        ));
    }
    Ok(())
}
