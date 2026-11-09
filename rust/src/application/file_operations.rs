use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    FileOperationEvent as Event, FileOperationPhase as Phase, FileOperationRecord,
    numbered_file_name,
};
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationIssue, FileOperationIssueKind as Kind,
    FileOperationResult,
};
use crate::ports::file_operation_journal::FileOperationJournal;
use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};

mod recovery;
pub use recovery::{FileOperationInspection, inspect_file_operation};

const MAX_NAME_CANDIDATES: u32 = 10_000;

pub struct FileOperationExecutor<'a> {
    journal: &'a mut dyn FileOperationJournal,
    native: &'a dyn NativeFileOperations,
    control: &'a FileOperationControl,
    now: &'a dyn Fn() -> i64,
}

impl<'a> FileOperationExecutor<'a> {
    pub fn new(
        journal: &'a mut dyn FileOperationJournal,
        native: &'a dyn NativeFileOperations,
        control: &'a FileOperationControl,
        now: &'a dyn Fn() -> i64,
    ) -> Self {
        Self {
            journal,
            native,
            control,
            now,
        }
    }

    pub fn execute(&mut self, id: u64) -> Result<FileOperationRecord, ScanError> {
        let mut record = self.journal.load(id)?;
        if !matches!(
            record.checkpoint.phase,
            Phase::Prepared | Phase::NameOccupied
        ) {
            return Err(ScanError::new(
                "file_operation_requires_inspection",
                "An interrupted operation must be inspected before another native action",
            ));
        }
        for number in 0..MAX_NAME_CANDIDATES {
            if self.control.is_cancelled() {
                return self.advance(&record, Event::Cancelled);
            }
            let destination = match self.destination(&record, number) {
                Ok(destination) => destination,
                Err(issue) => return self.failure(&record, issue),
            };
            if let Some(destination) = &destination {
                match self.native.destination_occupied(destination) {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(issue) => return self.failure(&record, issue),
                }
            }
            record = self.advance(&record, Event::Started(destination))?;
            let mut request = record.authorization.request.clone();
            request.destination = record.checkpoint.destination.clone();
            let result = self.native.execute(&request, self.control);
            match result {
                Err(issue) if issue.kind == Kind::Collision && request.destination.is_some() => {
                    record = self.advance(&record, Event::NameOccupied)?;
                }
                Err(issue)
                    if issue.kind == Kind::DifferentVolume
                        && request.action == FileAction::Move =>
                {
                    request.action = FileAction::Copy;
                    match self.native.execute(&request, self.control) {
                        Ok(receipt) => {
                            let destination = receipt.destination.ok_or_else(|| {
                                ScanError::new(
                                    "file_operation_missing_receipt",
                                    "A completed copy has no destination evidence",
                                )
                            })?;
                            record = self.advance(&record, Event::Copied(destination))?;
                            return self.finish_copied_move(record);
                        }
                        Err(issue) if issue.kind == Kind::Collision => {
                            record = self.advance(&record, Event::NameOccupied)?;
                        }
                        Err(issue) => return self.failure(&record, issue),
                    }
                }
                result => return self.finish(&record, result),
            }
        }
        self.failure(
            &record,
            FileOperationIssue::new(
                Kind::Collision,
                "No available filename was found within 10000 candidates",
            ),
        )
    }

    pub fn resume_copied_move(&mut self, id: u64) -> Result<FileOperationRecord, ScanError> {
        let record = self.journal.load(id)?;
        if !matches!(
            record.checkpoint.phase,
            Phase::Copied | Phase::Cancelled | Phase::Failed
        ) || !matches!(
            inspect_file_operation(&record, self.native),
            FileOperationInspection::CopyRetained
        ) {
            return Err(ScanError::new(
                "file_operation_requires_inspection",
                "Only a recorded copy can resume source removal",
            ));
        }
        self.finish_copied_move(record)
    }

    pub fn reconcile(&mut self, id: u64) -> Result<FileOperationRecord, ScanError> {
        let record = self.journal.load(id)?;
        match inspect_file_operation(&record, self.native) {
            FileOperationInspection::Completed(receipt)
                if record.checkpoint.phase != Phase::NativeCompleted =>
            {
                self.advance(&record, Event::NativeCompleted(receipt))
            }
            FileOperationInspection::CopyRetained
                if matches!(
                    record.checkpoint.phase,
                    Phase::RemovingSource | Phase::Uncertain
                ) =>
            {
                let copied = record.checkpoint.copied.clone().ok_or_else(|| {
                    ScanError::new(
                        "file_operation_missing_receipt",
                        "Recovery requires its recorded copy receipt",
                    )
                })?;
                self.advance(&record, Event::Copied(copied))
            }
            FileOperationInspection::NeedsAttention(issue)
                if record.checkpoint.phase != Phase::Uncertain =>
            {
                self.advance(&record, Event::Uncertain(issue))
            }
            _ => Ok(record),
        }
    }

    fn finish_copied_move(
        &mut self,
        record: FileOperationRecord,
    ) -> Result<FileOperationRecord, ScanError> {
        if self.control.is_cancelled() {
            return self.advance(&record, Event::Cancelled);
        }
        let destination = record.checkpoint.copied.clone().ok_or_else(|| {
            ScanError::new(
                "file_operation_missing_receipt",
                "Source removal requires its durable copy receipt",
            )
        })?;
        let record = self.advance(&record, Event::RemovingSource)?;
        let result = self.native.complete_move(
            &record.authorization.request.source,
            &destination,
            self.control,
        );
        self.finish(&record, result)
    }

    fn destination(
        &self,
        record: &FileOperationRecord,
        number: u32,
    ) -> Result<Option<FileOperationDestination>, FileOperationIssue> {
        let Some(original) = &record.authorization.request.destination else {
            return Ok(None);
        };
        let name = numbered_file_name(&original.name, number);
        if name.encode_utf16().count() > 255 {
            return Err(FileOperationIssue::new(
                Kind::InvalidRequest,
                "The numbered filename exceeds the Windows filename limit",
            ));
        }
        Ok(Some(FileOperationDestination {
            directory: original.directory.clone(),
            name,
        }))
    }

    fn finish(
        &mut self,
        record: &FileOperationRecord,
        result: FileOperationResult,
    ) -> Result<FileOperationRecord, ScanError> {
        match result {
            Ok(receipt) => self.advance(record, Event::NativeCompleted(receipt)),
            Err(issue) => self.failure(record, issue),
        }
    }

    fn failure(
        &mut self,
        record: &FileOperationRecord,
        issue: FileOperationIssue,
    ) -> Result<FileOperationRecord, ScanError> {
        let event = match issue.kind {
            Kind::Cancelled => Event::Cancelled,
            Kind::OutcomeUnknown => Event::Uncertain(issue),
            _ => Event::Failed(issue),
        };
        self.advance(record, event)
    }

    fn advance(
        &mut self,
        record: &FileOperationRecord,
        event: Event,
    ) -> Result<FileOperationRecord, ScanError> {
        self.journal
            .advance(record.id, record.checkpoint.version, event, (self.now)())
    }
}
