use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::BorrowedHandle;
use std::path::Path;
use std::sync::Mutex;

use windows_sys::Win32::Storage::FileSystem::{
    COPY_FILE_FAIL_IF_EXISTS, COPYFILE2_CALLBACK_CHUNK_FINISHED,
    COPYFILE2_CALLBACK_STREAM_FINISHED, COPYFILE2_CALLBACK_STREAM_STARTED,
    COPYFILE2_EXTENDED_PARAMETERS, COPYFILE2_MESSAGE, COPYFILE2_MESSAGE_ACTION,
    COPYFILE2_PROGRESS_CANCEL, COPYFILE2_PROGRESS_CONTINUE, CopyFile2, FlushFileBuffers,
};

use crate::adapters::local_files::operation_file_identity;
use crate::domain::FileIdentityEvidence;
use crate::ports::file_operations::FileOperationControl;

struct CopyContext<'a> {
    control: &'a FileOperationControl,
    destination: Mutex<Option<FileIdentityEvidence>>,
}

pub(super) fn copy(
    source: &Path,
    destination: &Path,
    control: &FileOperationControl,
) -> std::io::Result<FileIdentityEvidence> {
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let context = CopyContext {
        control,
        destination: Mutex::new(None),
    };
    let parameters = COPYFILE2_EXTENDED_PARAMETERS {
        dwSize: std::mem::size_of::<COPYFILE2_EXTENDED_PARAMETERS>() as u32,
        dwCopyFlags: COPY_FILE_FAIL_IF_EXISTS,
        pProgressRoutine: Some(progress),
        pvCallbackContext: std::ptr::from_ref(&context).cast_mut().cast(),
        ..Default::default()
    };
    // SAFETY: ADR 0042; paths, parameters and context remain live for the synchronous call.
    // The callback copies file identity and progress, retains no native pointers and cannot unwind.
    let result = unsafe { CopyFile2(source.as_ptr(), destination.as_ptr(), &parameters) };
    if result != 0 {
        return Err(std::io::Error::from_raw_os_error(result & 0xffff));
    }
    context
        .destination
        .into_inner()
        .map_err(|_| std::io::Error::other("Copy identity recording failed"))?
        .ok_or_else(|| {
            std::io::Error::other(
                "Windows completed the copy without destination identity evidence",
            )
        })
}

unsafe extern "system" fn progress(
    message: *const COPYFILE2_MESSAGE,
    context: *const c_void,
) -> COPYFILE2_MESSAGE_ACTION {
    if message.is_null() || context.is_null() {
        return COPYFILE2_PROGRESS_CANCEL;
    }
    // SAFETY: CopyFile2 supplies this message and the live context for this callback only.
    let (message, context) = unsafe { (&*message, &*context.cast::<CopyContext<'_>>()) };
    if message.Type == COPYFILE2_CALLBACK_STREAM_STARTED {
        // SAFETY: the tag selects StreamStarted. The callback borrows Windows' file handle;
        // a cloned handle is closed after obtaining its identity, before returning to Windows.
        let handle = unsafe { message.Info.StreamStarted.hDestinationFile };
        if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return COPYFILE2_PROGRESS_CANCEL;
        }
        let identity = unsafe { BorrowedHandle::borrow_raw(handle) }
            .try_clone_to_owned()
            .and_then(|handle| operation_file_identity(&std::fs::File::from(handle)));
        let (Ok(identity), Ok(mut destination)) = (identity, context.destination.lock()) else {
            return COPYFILE2_PROGRESS_CANCEL;
        };
        if destination
            .as_ref()
            .is_some_and(|previous| previous != &identity)
        {
            return COPYFILE2_PROGRESS_CANCEL;
        }
        *destination = Some(identity);
    }
    if message.Type == COPYFILE2_CALLBACK_CHUNK_FINISHED {
        // SAFETY: the message tag selects the ChunkFinished union member.
        let chunk = unsafe { message.Info.ChunkFinished };
        context
            .control
            .publish_copy_progress(chunk.uliTotalBytesTransferred, chunk.uliTotalFileSize);
    }
    if message.Type == COPYFILE2_CALLBACK_STREAM_FINISHED {
        // SAFETY: the tag selects the live writable destination handle. Flush this stream
        // before returning a copy receipt that may authorize a later verified move completion.
        if unsafe { FlushFileBuffers(message.Info.StreamFinished.hDestinationFile) } == 0 {
            return COPYFILE2_PROGRESS_CANCEL;
        }
    }
    if context.control.is_cancelled() {
        COPYFILE2_PROGRESS_CANCEL
    } else {
        COPYFILE2_PROGRESS_CONTINUE
    }
}
