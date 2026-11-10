mod complete_move;
mod copy;
mod exports;
mod output;
mod recovery;
mod remove;
mod rename;
mod scope;
mod shell_delete;

use std::sync::{Mutex, MutexGuard, TryLockError};

use crate::domain::file_operations::{
    FileAction, FileOperationContentProof, FileOperationDestination, FileOperationDirectory,
    FileOperationIssue, FileOperationIssueKind as Kind, FileOperationReceipt,
    FileOperationRecovery, FileOperationRequest, FileOperationResult, FileOperationSource,
};
use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};

use scope::OperationScope;

static MUTATION_LANE: Mutex<()> = Mutex::new(());

pub struct WindowsFileOperations {
    owner_window: Option<usize>,
}

impl WindowsFileOperations {
    pub fn new(owner_window: Option<usize>) -> Self {
        Self { owner_window }
    }
}

impl NativeFileOperations for WindowsFileOperations {
    fn inspect_source(&self, path: &str) -> Result<FileOperationSource, FileOperationIssue> {
        scope::inspect_source(path)
    }

    fn inspect_directory(&self, path: &str) -> Result<FileOperationDirectory, FileOperationIssue> {
        scope::inspect_directory(path)
    }

    fn destination_occupied(
        &self,
        destination: &FileOperationDestination,
    ) -> Result<bool, FileOperationIssue> {
        scope::destination_occupied(destination)
    }

    fn inspect_location(
        &self,
        location: &FileOperationDestination,
    ) -> Result<Option<FileOperationSource>, FileOperationIssue> {
        scope::inspect_location(location)
    }

    fn verify_content(
        &self,
        source: &FileOperationSource,
        proof: &FileOperationContentProof,
        control: &FileOperationControl,
    ) -> Result<(), FileOperationIssue> {
        recovery::verify_content(source, proof, control)
    }

    fn complete_move(
        &self,
        source: &FileOperationSource,
        destination: &FileOperationSource,
        control: &FileOperationControl,
    ) -> FileOperationResult {
        let _lane = mutation_lane()?;
        if control.is_cancelled() {
            return Err(cancelled());
        }
        complete_move::complete_move(source, destination, control)
    }

    fn execute(
        &self,
        request: &FileOperationRequest,
        control: &FileOperationControl,
    ) -> FileOperationResult {
        let _lane = mutation_lane()?;
        if control.is_cancelled() {
            return Err(cancelled());
        }
        let scope = OperationScope::open(request)?;
        if request.action == FileAction::DeletePermanently {
            return remove::permanently(request, &scope, control);
        }
        if request.action == FileAction::Recycle {
            return std::thread::scope(|threads| {
                std::thread::Builder::new()
                    .name("ame-file-operation".to_owned())
                    .spawn_scoped(threads, || {
                        shell_delete::delete(
                            request.clone(),
                            scope,
                            control.clone(),
                            self.owner_window,
                        )
                    })
                    .map_err(|error| FileOperationIssue::new(Kind::Unavailable, error.to_string()))?
                    .join()
                    .unwrap_or_else(|_| {
                        Err(FileOperationIssue::new(
                            Kind::OutcomeUnknown,
                            "The native deletion owner terminated without a result",
                        ))
                    })
            });
        }
        transfer(request, &scope, control)
    }
}

fn mutation_lane() -> Result<MutexGuard<'static, ()>, FileOperationIssue> {
    MUTATION_LANE.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => FileOperationIssue::new(
            Kind::Unavailable,
            "Another native operation is still active",
        ),
        TryLockError::Poisoned(_) => FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "The native operation lane requires restart reconciliation",
        ),
    })
}

fn transfer(
    request: &FileOperationRequest,
    scope: &OperationScope,
    control: &FileOperationControl,
) -> FileOperationResult {
    let (Some(destination), Some(target)) = (&request.destination, &scope.destination) else {
        return Err(FileOperationIssue::new(
            Kind::InvalidRequest,
            "Missing destination",
        ));
    };
    let moving = request.action != FileAction::Copy;
    let source = scope
        .source
        .open_operation_source(&scope.source_name, &request.source.state, moving)
        .map_err(|error| FileOperationIssue::new(Kind::SourceChanged, error.to_string()))?;
    if control.is_cancelled() {
        return Err(cancelled());
    }
    let destination_path = target.operation_path(&destination.name);
    let result = if moving {
        rename::rename(&source, &destination_path)
            .map(|()| request.source.state.file_identity.clone())
    } else {
        control.publish_copy_progress(0, request.source.state.file_size);
        copy::copy(
            &scope.source.operation_path(&scope.source_name),
            &destination_path,
            control,
        )
        .map(Some)
    };
    let destination_identity = match result {
        Ok(identity) => identity,
        Err(error) => {
            let kind = match error.raw_os_error() {
                Some(80 | 183) => Kind::Collision,
                Some(17) => Kind::DifferentVolume,
                Some(1235) if control.is_cancelled() => Kind::Cancelled,
                _ => Kind::NativeFailure,
            };
            // Failed native copies can leave a partial target. Never retry that path as a new item.
            let kind = if !moving && kind != Kind::Collision {
                match destination_path.symlink_metadata() {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => kind,
                    _ => Kind::OutcomeUnknown,
                }
            } else {
                kind
            };
            return Err(FileOperationIssue::new(kind, error.to_string()));
        }
    };
    let observed = scope::inspect_guarded(target, &destination.directory, &destination.name)
        .map_err(|error| FileOperationIssue::new(Kind::OutcomeUnknown, error.message))?;
    if observed.state.file_size != request.source.state.file_size
        || observed.state.file_identity != destination_identity
    {
        return Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "The native destination no longer matches the completed operation",
        ));
    }
    drop(source);
    Ok(FileOperationReceipt {
        destination: Some(observed),
        recovery: FileOperationRecovery::None,
    })
}

fn cancelled() -> FileOperationIssue {
    FileOperationIssue::new(
        Kind::Cancelled,
        "The operation was cancelled before submission",
    )
}
