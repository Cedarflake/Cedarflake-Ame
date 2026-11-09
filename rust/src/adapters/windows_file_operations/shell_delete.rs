use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{E_ABORT, E_FAIL, HWND};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::{
    COPYENGINE_S_DONT_PROCESS_CHILDREN, FOF_SILENT, FOF_WANTNUKEWARNING, FOFX_RECYCLEONDELETE,
    FileOperation, IFileOperation, IFileOperationProgressSink, IFileOperationProgressSink_Impl,
    IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
};
use windows::core::{HRESULT, HSTRING, PCWSTR, Ref, Result, implement};

use crate::domain::file_operations::{
    FileAction, FileOperationIssue, FileOperationIssueKind as Kind, FileOperationReceipt,
    FileOperationRecovery, FileOperationRequest, FileOperationResult,
};
use crate::ports::file_operations::FileOperationControl;

use super::scope::{self, OperationScope};

struct Apartment;

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: created only after successful initialization on this dedicated STA;
        // operation and callback interfaces have already been dropped on the same thread.
        unsafe {
            CoUninitialize();
        }
    }
}

struct AdvisedOperation {
    operation: IFileOperation,
    cookie: u32,
}

impl Drop for AdvisedOperation {
    fn drop(&mut self) {
        // SAFETY: the operation and sink remain owned by this apartment through retirement.
        let _ = unsafe { self.operation.Unadvise(self.cookie) };
    }
}

#[derive(Default)]
struct DeletionResult {
    pre_issue: Option<FileOperationIssue>,
    item_status: Option<HRESULT>,
    recycled: bool,
}

struct Admission {
    request: FileOperationRequest,
    scope: OperationScope,
    control: FileOperationControl,
}

#[implement(IFileOperationProgressSink)]
struct Sink {
    admission: Arc<Admission>,
    result: Arc<Mutex<DeletionResult>>,
}

impl Sink {
    fn admit(&self, item: Ref<IShellItem>) -> Result<()> {
        let check = (|| {
            if self.admission.control.is_cancelled() {
                return Err(FileOperationIssue::new(
                    Kind::Cancelled,
                    "Deletion was cancelled before submission",
                ));
            }
            self.admission.scope.revalidate(&self.admission.request)?;
            let item = item.as_ref().ok_or_else(|| {
                FileOperationIssue::new(Kind::SourceChanged, "Windows supplied no source item")
            })?;
            let path = display_path(item).map_err(native_error)?;
            let actual = scope::inspect_source(&path)?;
            let expected = &self.admission.request.source.state;
            if actual.state.file_identity != expected.file_identity
                || actual.state.source_revision != expected.source_revision
            {
                return Err(FileOperationIssue::new(
                    Kind::SourceChanged,
                    "The native source no longer matches the authorized file",
                ));
            }
            Ok(())
        })();
        if let Err(issue) = check {
            self.result
                .lock()
                .map_err(|_| windows::core::Error::from(E_FAIL))?
                .pre_issue = Some(issue);
            return Err(E_ABORT.into());
        }
        Ok(())
    }
}

#[allow(non_snake_case)]
impl IFileOperationProgressSink_Impl for Sink_Impl {
    fn StartOperations(&self) -> Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _: HRESULT) -> Result<()> {
        Ok(())
    }
    fn PreDeleteItem(&self, _: u32, item: Ref<IShellItem>) -> Result<()> {
        self.admit(item)
    }
    fn PostDeleteItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        status: HRESULT,
        recycled: Ref<IShellItem>,
    ) -> Result<()> {
        let mut result = self
            .result
            .lock()
            .map_err(|_| windows::core::Error::from(E_FAIL))?;
        result.item_status = Some(status);
        result.recycled = recycled.as_ref().is_some();
        Ok(())
    }
    fn PreRenameItem(&self, _: u32, _: Ref<IShellItem>, _: &PCWSTR) -> Result<()> {
        Err(E_ABORT.into())
    }
    fn PostRenameItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreMoveItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
    ) -> Result<()> {
        Err(E_ABORT.into())
    }
    fn PostMoveItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreCopyItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
    ) -> Result<()> {
        Err(E_ABORT.into())
    }
    fn PostCopyItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreNewItem(&self, _: u32, _: Ref<IShellItem>, _: &PCWSTR) -> Result<()> {
        Err(E_ABORT.into())
    }
    fn PostNewItem(
        &self,
        _: u32,
        _: Ref<IShellItem>,
        _: &PCWSTR,
        _: &PCWSTR,
        _: u32,
        _: HRESULT,
        _: Ref<IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, _: u32, _: u32) -> Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> Result<()> {
        Ok(())
    }
}

pub(super) fn delete(
    request: FileOperationRequest,
    scope: OperationScope,
    control: FileOperationControl,
    owner_window: Option<usize>,
) -> FileOperationResult {
    // SAFETY: ADR 0042; this function runs only on its dedicated joined thread. The
    // apartment guard is created first and is therefore dropped after every COM interface.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }.map_err(native_error)?;
    let _apartment = Apartment;
    let admission = Arc::new(Admission {
        request,
        scope,
        control,
    });
    let result = Arc::new(Mutex::new(DeletionResult::default()));
    let sink: IFileOperationProgressSink = Sink {
        admission: admission.clone(),
        result: result.clone(),
    }
    .into();
    // SAFETY: all interface calls are apartment-local, and UTF-16 buffers remain live for
    // their calls. Windows owns dialogs; no callback answers a permanent-delete prompt.
    let (status, aborted) = unsafe {
        let operation: IFileOperation =
            CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER).map_err(native_error)?;
        let mut flags = FOF_SILENT | FOF_WANTNUKEWARNING;
        if admission.request.action == FileAction::Recycle {
            flags |= FOFX_RECYCLEONDELETE;
        }
        operation.SetOperationFlags(flags).map_err(native_error)?;
        if let Some(window) = owner_window {
            validate_window(window)?;
            operation
                .SetOwnerWindow(HWND(window as *mut _))
                .map_err(native_error)?;
        }
        let cookie = operation.Advise(&sink).map_err(native_error)?;
        let operation = AdvisedOperation { operation, cookie };
        let parsing_name =
            crate::domain::user_visible_path(&admission.request.source.state.absolute_path);
        let source: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(parsing_name), None)
            .map_err(|error| {
                FileOperationIssue::new(
                    Kind::NativeFailure,
                    format!("Windows could not resolve the file for deletion: {error}"),
                )
            })?;
        operation
            .operation
            .DeleteItem(&source, None)
            .map_err(native_error)?;
        let status = operation.operation.PerformOperations();
        let aborted = operation
            .operation
            .GetAnyOperationsAborted()
            .map_err(|error| FileOperationIssue::new(Kind::OutcomeUnknown, error.to_string()))?
            .as_bool();
        (status, aborted)
    };
    drop(sink);
    let result = result.lock().map_err(|_| {
        FileOperationIssue::new(Kind::OutcomeUnknown, "The native result could not be read")
    })?;
    if let Some(issue) = &result.pre_issue {
        return Err(issue.clone());
    }
    let completed = result
        .item_status
        .is_some_and(|code| code.0 == 0 || code == COPYENGINE_S_DONT_PROCESS_CHILDREN);
    if completed && status.is_ok() && !aborted {
        let path = admission
            .scope
            .source
            .operation_path(&admission.scope.source_name);
        match path.symlink_metadata() {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FileOperationReceipt {
                    destination: None,
                    recovery: if result.recycled {
                        FileOperationRecovery::SystemRecycleBin
                    } else {
                        FileOperationRecovery::None
                    },
                });
            }
            _ => {
                return Err(FileOperationIssue::new(
                    Kind::OutcomeUnknown,
                    "The deleted path needs reconciliation",
                ));
            }
        }
    }
    if admission.scope.revalidate(&admission.request).is_err() {
        return Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "The source changed before a conclusive deletion result was recorded",
        ));
    }
    if aborted || admission.control.is_cancelled() {
        Err(FileOperationIssue::new(
            Kind::Cancelled,
            "Windows cancelled the deletion",
        ))
    } else {
        Err(FileOperationIssue::new(
            Kind::NativeFailure,
            status.err().map_or_else(
                || "Windows did not complete the item".to_owned(),
                |error| error.to_string(),
            ),
        ))
    }
}

fn display_path(item: &IShellItem) -> Result<String> {
    // SAFETY: Shell owns this terminated string; copy it before releasing its allocation.
    unsafe {
        let value = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let text = value.to_string();
        CoTaskMemFree(Some(value.0.cast()));
        Ok(text?)
    }
}

fn validate_window(window: usize) -> std::result::Result<(), FileOperationIssue> {
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut owner = 0;
    // SAFETY: Windows validates the opaque window handle and writes into a live local DWORD.
    let valid = unsafe {
        GetWindowThreadProcessId(window as *mut _, &mut owner) != 0
            && owner == GetCurrentProcessId()
    };
    if valid {
        Ok(())
    } else {
        Err(FileOperationIssue::new(
            Kind::InvalidRequest,
            "The dialog owner is not a live window in this process",
        ))
    }
}

fn native_error(error: windows::core::Error) -> FileOperationIssue {
    FileOperationIssue::new(Kind::NativeFailure, error.to_string())
}
