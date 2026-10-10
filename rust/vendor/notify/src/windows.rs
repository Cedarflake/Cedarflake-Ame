#![allow(missing_docs)]
//! Watcher implementation for Windows' directory management APIs
//!
//! For more information see the [ReadDirectoryChangesW reference][ref].
//!
//! [ref]: https://msdn.microsoft.com/en-us/library/windows/desktop/aa363950(v=vs.85).aspx

use crate::{bounded, unbounded, BoundSender, Config, Receiver, Sender};
use crate::{event::*, WatcherKind};
use crate::{Error, EventHandler, RecursiveMode, Result, Watcher};
use std::alloc;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::os::raw::c_void;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_NOTIFY_ENUM_DIR, ERROR_OPERATION_ABORTED,
    ERROR_SUCCESS, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FileStandardInfo, GetFileInformationByHandleEx, ReadDirectoryChangesW,
    FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_CREATION,
    FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SECURITY, FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, OPEN_EXISTING,
};
use windows_sys::Win32::System::Threading::{
    CreateSemaphoreW, ReleaseSemaphore, WaitForSingleObjectEx, INFINITE,
};
use windows_sys::Win32::System::IO::{CancelIo, OVERLAPPED};

const BUF_SIZE: u32 = 16384;

#[derive(Clone)]
struct ReadData {
    dir: PathBuf,          // directory that is being watched
    file: Option<PathBuf>, // if a file is being watched, this is its full path
    complete_sem: HANDLE,
    is_recursive: bool,
    stopping: Arc<AtomicBool>,
}

struct ReadDirectoryRequest {
    event_handler: Arc<Mutex<dyn EventHandler>>,
    buffer: [u8; BUF_SIZE as usize],
    handle: HANDLE,
    data: ReadData,
    action_tx: Sender<Action>,
}

impl ReadDirectoryRequest {
    fn unwatch(&self) {
        let _ = self.action_tx.send(Action::Unwatch(self.data.dir.clone()));
    }
}

enum Action {
    Watch(PathBuf, RecursiveMode),
    Unwatch(PathBuf),
    Stop,
    Configure(Config, BoundSender<Result<bool>>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MetaEvent {
    SingleWatchComplete,
    WatcherAwakened,
}

struct WatchState {
    dir_handle: HANDLE,
    complete_sem: HANDLE,
    stopping: Arc<AtomicBool>,
}

struct ReadDirectoryChangesServer {
    tx: Sender<Action>,
    rx: Receiver<Action>,
    event_handler: Arc<Mutex<dyn EventHandler>>,
    meta_tx: Sender<MetaEvent>,
    cmd_tx: Sender<Result<PathBuf>>,
    watches: HashMap<PathBuf, WatchState>,
    wakeup_sem: HANDLE,
}

impl ReadDirectoryChangesServer {
    fn start(
        event_handler: Arc<Mutex<dyn EventHandler>>,
        meta_tx: Sender<MetaEvent>,
        cmd_tx: Sender<Result<PathBuf>>,
        wakeup_sem: HANDLE,
    ) -> Result<(Sender<Action>, JoinHandle<()>)> {
        let (action_tx, action_rx) = unbounded();
        // it is, in fact, ok to send the semaphore across threads
        let sem_temp = wakeup_sem as u64;
        let server_thread = thread::Builder::new()
            .name("notify-rs windows loop".to_string())
            .spawn({
                let tx = action_tx.clone();
                move || {
                    let wakeup_sem = sem_temp as HANDLE;
                    let server = ReadDirectoryChangesServer {
                        tx,
                        rx: action_rx,
                        event_handler,
                        meta_tx,
                        cmd_tx,
                        watches: HashMap::new(),
                        wakeup_sem,
                    };
                    server.run();
                }
            })
            .map_err(|error| Error::io(error))?;
        Ok((action_tx, server_thread))
    }

    fn run(mut self) {
        loop {
            // process all available actions first
            let mut stopped = false;

            while let Ok(action) = self.rx.try_recv() {
                match action {
                    Action::Watch(path, recursive_mode) => {
                        let res = self.add_watch(path, recursive_mode.is_recursive());
                        let _ = self.cmd_tx.send(res);
                    }
                    Action::Unwatch(path) => self.remove_watch(path),
                    Action::Stop => {
                        stopped = true;
                        for ws in self.watches.values() {
                            stop_watch(ws, &self.meta_tx);
                        }
                        break;
                    }
                    Action::Configure(config, tx) => {
                        self.configure_raw_mode(config, tx);
                    }
                }
            }

            if stopped {
                break;
            }

            unsafe {
                // wait with alertable flag so that the completion routine fires
                let waitres = WaitForSingleObjectEx(self.wakeup_sem, 100, 1);
                if waitres == WAIT_OBJECT_0 {
                    let _ = self.meta_tx.send(MetaEvent::WatcherAwakened);
                }
            }
        }

    }

    fn add_watch(&mut self, path: PathBuf, is_recursive: bool) -> Result<PathBuf> {
        // path must exist and be either a file or directory
        if !path.is_dir() && !path.is_file() {
            return Err(
                Error::generic("Input watch path is neither a file nor a directory.")
                    .add_path(path),
            );
        }

        let (watching_file, dir_target) = {
            if path.is_dir() {
                (false, path.clone())
            } else {
                // emulate file watching by watching the parent directory
                (true, path.parent().unwrap().to_path_buf())
            }
        };

        let encoded_path: Vec<u16> = dir_target
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let handle;
        unsafe {
            handle = CreateFileW(
                encoded_path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_DELETE | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(if watching_file {
                    Error::generic(
                        "You attempted to watch a single file, but parent \
                         directory could not be opened.",
                    )
                    .add_path(path)
                } else {
                    // TODO: Call GetLastError for better error info?
                    Error::path_not_found().add_path(path)
                });
            }
        }
        let wf = if watching_file {
            Some(path.clone())
        } else {
            None
        };
        // every watcher gets its own semaphore to signal completion
        let semaphore = unsafe { CreateSemaphoreW(ptr::null_mut(), 0, 1, ptr::null_mut()) };
        if semaphore.is_null() || semaphore == INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(handle);
            }
            return Err(Error::generic("Failed to create semaphore for watch.").add_path(path));
        }
        let stopping = Arc::new(AtomicBool::new(false));
        let rd = ReadData {
            dir: dir_target,
            file: wf,
            complete_sem: semaphore,
            is_recursive,
            stopping: Arc::clone(&stopping),
        };
        let ws = WatchState {
            dir_handle: handle,
            complete_sem: semaphore,
            stopping,
        };
        self.watches.insert(path.clone(), ws);
        if let Err(error) = start_read(&rd, self.event_handler.clone(), handle, self.tx.clone()) {
            self.watches.remove(&path);
            unsafe {
                CloseHandle(handle);
                CloseHandle(semaphore);
            }
            return Err(error);
        }
        Ok(path)
    }

    fn remove_watch(&mut self, path: PathBuf) {
        if let Some(ws) = self.watches.remove(&path) {
            stop_watch(&ws, &self.meta_tx);
        }
    }

    fn configure_raw_mode(&mut self, _config: Config, tx: BoundSender<Result<bool>>) {
        tx.send(Ok(false))
            .expect("configuration channel disconnect");
    }
}

fn stop_watch(ws: &WatchState, meta_tx: &Sender<MetaEvent>) {
    // A successful completion may already be queued when cancellation starts. Mark the watch
    // first so that callback does not rearm ReadDirectoryChangesW with the handle closed below.
    ws.stopping.store(true, Ordering::Release);
    unsafe {
        let cio = CancelIo(ws.dir_handle);
        let ch = CloseHandle(ws.dir_handle);
        // have to wait for it, otherwise we leak the memory allocated for there read request
        if cio != 0 && ch != 0 {
            while WaitForSingleObjectEx(ws.complete_sem, INFINITE, 1) != WAIT_OBJECT_0 {
                // drain the apc queue, fix for https://github.com/notify-rs/notify/issues/287#issuecomment-801465550
            }
        }
        CloseHandle(ws.complete_sem);
    }
    let _ = meta_tx.send(MetaEvent::SingleWatchComplete);
}

fn start_read(
    rd: &ReadData,
    event_handler: Arc<Mutex<dyn EventHandler>>,
    handle: HANDLE,
    action_tx: Sender<Action>,
) -> Result<()> {
    let request = Box::new(ReadDirectoryRequest {
        event_handler,
        handle,
        buffer: [0u8; BUF_SIZE as usize],
        data: rd.clone(),
        action_tx,
    });

    let flags = FILE_NOTIFY_CHANGE_FILE_NAME
        | FILE_NOTIFY_CHANGE_DIR_NAME
        | FILE_NOTIFY_CHANGE_ATTRIBUTES
        | FILE_NOTIFY_CHANGE_SIZE
        | FILE_NOTIFY_CHANGE_LAST_WRITE
        | FILE_NOTIFY_CHANGE_CREATION
        | FILE_NOTIFY_CHANGE_SECURITY;

    let monitor_subdir = if request.data.file.is_none() && request.data.is_recursive {
        1
    } else {
        0
    };

    unsafe {
        let overlapped = alloc::alloc_zeroed(alloc::Layout::new::<OVERLAPPED>()) as *mut OVERLAPPED;
        // When using callback based async requests, we are allowed to use the hEvent member
        // for our own purposes

        let request = Box::leak(request);
        (*overlapped).hEvent = request as *mut _ as _;

        // This is using an asynchronous call with a completion routine for receiving notifications
        // An I/O completion port would probably be more performant
        let ret = ReadDirectoryChangesW(
            handle,
            request.buffer.as_mut_ptr() as *mut c_void,
            BUF_SIZE,
            monitor_subdir,
            flags,
            &mut 0u32 as *mut u32, // not used for async reqs
            overlapped,
            Some(handle_event),
        );

        if ret == 0 {
            let error = std::io::Error::last_os_error();
            // error reading. retransmute request memory to allow drop.
            // Because of the error, ownership of the `overlapped` alloc was not passed
            // over to `ReadDirectoryChangesW`.
            // So we can claim ownership back.
            let _overlapped = Box::from_raw(overlapped);
            let request = Box::from_raw(request);
            let path = request
                .data
                .file
                .clone()
                .unwrap_or_else(|| request.data.dir.clone());
            ReleaseSemaphore(request.data.complete_sem, 1, ptr::null_mut());
            return Err(Error::io(error).add_path(path));
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompletionDisposition {
    Aborted,
    RootRemoved,
    Error,
    Rescan,
    Records,
}

fn completion_disposition(
    error_code: u32,
    bytes_written: u32,
    root_exists: Option<bool>,
    is_delete_pending: bool,
) -> CompletionDisposition {
    match error_code {
        ERROR_OPERATION_ABORTED => CompletionDisposition::Aborted,
        ERROR_ACCESS_DENIED if root_exists == Some(false) || is_delete_pending => {
            CompletionDisposition::RootRemoved
        }
        ERROR_ACCESS_DENIED => CompletionDisposition::Error,
        ERROR_NOTIFY_ENUM_DIR => CompletionDisposition::Rescan,
        ERROR_SUCCESS if bytes_written == 0 => CompletionDisposition::Rescan,
        ERROR_SUCCESS => CompletionDisposition::Records,
        _ => CompletionDisposition::Error,
    }
}

fn is_delete_pending(handle: HANDLE) -> bool {
    let mut info = FILE_STANDARD_INFO::default();
    let success = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileStandardInfo,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of_val(&info) as u32,
        )
    };
    success != 0 && info.DeletePending
}

fn emit_event(event_handler: &Mutex<dyn EventHandler>, result: Result<Event>) {
    if let Ok(mut guard) = event_handler.lock() {
        let handler: &mut dyn EventHandler = &mut *guard;
        handler.handle_event(result);
    }
}

unsafe extern "system" fn handle_event(
    error_code: u32,
    bytes_written: u32,
    overlapped: *mut OVERLAPPED,
) {
    let overlapped: Box<OVERLAPPED> = Box::from_raw(overlapped);
    let request: Box<ReadDirectoryRequest> = Box::from_raw(overlapped.hEvent as *mut _);

    // CancelIo does not change a completion that was already queued successfully. Such a callback
    // can run while stop_watch is waiting, after it has closed the directory handle.
    if request.data.stopping.load(Ordering::Acquire) {
        ReleaseSemaphore(request.data.complete_sem, 1, ptr::null_mut());
        return;
    }

    let root_exists = (error_code == ERROR_ACCESS_DENIED)
        .then(|| request.data.dir.try_exists().ok())
        .flatten();
    let delete_pending = error_code == ERROR_ACCESS_DENIED && is_delete_pending(request.handle);
    let disposition =
        completion_disposition(error_code, bytes_written, root_exists, delete_pending);
    match disposition {
        CompletionDisposition::Aborted => {
            // received when dir is unwatched or watcher is shutdown; return and let overlapped/request get drop-cleaned
            ReleaseSemaphore(request.data.complete_sem, 1, ptr::null_mut());
            return;
        }
        CompletionDisposition::RootRemoved => {
            if request.data.file.is_none() {
                let event = Event::new(EventKind::Remove(RemoveKind::Folder))
                    .add_path(request.data.dir.clone());
                emit_event(&request.event_handler, Ok(event));
            }
            request.unwatch();
            ReleaseSemaphore(request.data.complete_sem, 1, ptr::null_mut());
            return;
        }
        CompletionDisposition::Error => {
            let error = Error::io(std::io::Error::from_raw_os_error(
                i32::try_from(error_code).unwrap_or(i32::MAX),
            ))
            .add_path(request.data.dir.clone());
            emit_event(&request.event_handler, Err(error));
            request.unwatch();
            ReleaseSemaphore(request.data.complete_sem, 1, ptr::null_mut());
            return;
        }
        CompletionDisposition::Rescan | CompletionDisposition::Records => {
            // Continue below to rearm before handling the completion.
        }
    }

    // Get the next request queued up as soon as possible
    let rearm_error = start_read(
        &request.data,
        request.event_handler.clone(),
        request.handle,
        request.action_tx.clone(),
    )
    .err();

    if disposition == CompletionDisposition::Rescan {
        emit_event(
            &request.event_handler,
            Ok(Event::new(EventKind::Other).set_flag(Flag::Rescan)),
        );
        if let Some(error) = rearm_error {
            emit_event(&request.event_handler, Err(error));
            request.unwatch();
        }
        return;
    }

    // The FILE_NOTIFY_INFORMATION struct has a variable length due to the variable length
    // string as its last member. Each struct contains an offset for getting the next entry in
    // the buffer.
    let mut cur_offset: *const u8 = request.buffer.as_ptr();
    // In Wine, FILE_NOTIFY_INFORMATION structs are packed placed in the buffer;
    // they are aligned to 16bit (WCHAR) boundary instead of 32bit required by FILE_NOTIFY_INFORMATION.
    // Hence, we need to use `read_unaligned` here to avoid UB.
    let mut cur_entry = ptr::read_unaligned(cur_offset as *const FILE_NOTIFY_INFORMATION);
    loop {
        // filename length is size in bytes, so / 2
        let len = cur_entry.FileNameLength as usize / 2;
        let encoded_path: &[u16] = slice::from_raw_parts(
            cur_offset.offset(std::mem::offset_of!(FILE_NOTIFY_INFORMATION, FileName) as isize)
                as _,
            len,
        );
        // prepend root to get a full path
        let path = request
            .data
            .dir
            .join(PathBuf::from(OsString::from_wide(encoded_path)));

        // if we are watching a single file, ignore the event unless the path is exactly
        // the watched file
        let skip = match request.data.file {
            None => false,
            Some(ref watch_path) => *watch_path != path,
        };

        if !skip {
            log::trace!(
                "Event: path = `{}`, action = {:?}",
                path.display(),
                cur_entry.Action
            );

            let newe = Event::new(EventKind::Any).add_path(path);

            let event_handler = |res| emit_event(&request.event_handler, res);

            if cur_entry.Action == FILE_ACTION_RENAMED_OLD_NAME {
                let mode = RenameMode::From;
                let kind = ModifyKind::Name(mode);
                let kind = EventKind::Modify(kind);
                let ev = newe.set_kind(kind);
                event_handler(Ok(ev))
            } else {
                match cur_entry.Action {
                    FILE_ACTION_RENAMED_NEW_NAME => {
                        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::To));
                        let ev = newe.set_kind(kind);
                        event_handler(Ok(ev));
                    }
                    FILE_ACTION_ADDED => {
                        let kind = EventKind::Create(CreateKind::Any);
                        let ev = newe.set_kind(kind);
                        event_handler(Ok(ev));
                    }
                    FILE_ACTION_REMOVED => {
                        let kind = EventKind::Remove(RemoveKind::Any);
                        let ev = newe.set_kind(kind);
                        event_handler(Ok(ev));
                    }
                    FILE_ACTION_MODIFIED => {
                        let kind = EventKind::Modify(ModifyKind::Any);
                        let ev = newe.set_kind(kind);
                        event_handler(Ok(ev));
                    }
                    _ => (),
                };
            }
        }

        if cur_entry.NextEntryOffset == 0 {
            break;
        }
        cur_offset = cur_offset.offset(cur_entry.NextEntryOffset as isize);
        cur_entry = ptr::read_unaligned(cur_offset as *const FILE_NOTIFY_INFORMATION);
    }

    if let Some(error) = rearm_error {
        emit_event(&request.event_handler, Err(error));
        request.unwatch();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::ptr;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Condvar, Mutex, mpsc};
    use std::thread;
    use std::time::Duration;

    use super::{
        BUF_SIZE, ReadData, ReadDirectoryChangesWatcher, ReadDirectoryRequest,
        completion_disposition, CompletionDisposition, ERROR_ACCESS_DENIED, ERROR_NOTIFY_ENUM_DIR,
        ERROR_SUCCESS, handle_event,
    };
    use crate::{RecursiveMode, Watcher};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::IO::OVERLAPPED;
    use windows_sys::Win32::System::Threading::{CreateSemaphoreW, WaitForSingleObjectEx};

    #[test]
    fn access_denied_with_an_existing_root_is_an_error_not_records() {
        assert_eq!(
            completion_disposition(ERROR_ACCESS_DENIED, 0, Some(true), false),
            CompletionDisposition::Error
        );
    }

    #[test]
    fn access_denied_with_an_unknown_root_state_is_an_error_not_records() {
        assert_eq!(
            completion_disposition(ERROR_ACCESS_DENIED, 0, None, false),
            CompletionDisposition::Error
        );
    }

    #[test]
    fn only_nonempty_success_can_enter_the_record_parser() {
        assert_eq!(
            completion_disposition(ERROR_SUCCESS, 1, None, false),
            CompletionDisposition::Records
        );
        assert_eq!(
            completion_disposition(ERROR_SUCCESS, 0, None, false),
            CompletionDisposition::Rescan
        );
        assert_eq!(
            completion_disposition(ERROR_NOTIFY_ENUM_DIR, 1, None, false),
            CompletionDisposition::Rescan
        );
    }

    #[test]
    fn delete_pending_root_is_removed_even_while_its_path_remains_visible() {
        assert_eq!(
            completion_disposition(ERROR_ACCESS_DENIED, 0, Some(true), true),
            CompletionDisposition::RootRemoved
        );
    }

    #[test]
    fn stopped_watch_does_not_rearm_queued_successful_completion() {
        let complete_sem = unsafe { CreateSemaphoreW(ptr::null_mut(), 0, 1, ptr::null_mut()) };
        assert!(!complete_sem.is_null());
        assert_ne!(complete_sem, INVALID_HANDLE_VALUE);

        let (event_tx, event_rx) = mpsc::channel();
        let (action_tx, action_rx) = crate::unbounded();
        let event_handler: Arc<Mutex<dyn crate::EventHandler>> = Arc::new(Mutex::new(event_tx));
        let request = Box::new(ReadDirectoryRequest {
            event_handler,
            buffer: [0u8; BUF_SIZE as usize],
            handle: INVALID_HANDLE_VALUE,
            data: ReadData {
                dir: PathBuf::from(r"C:\watched"),
                file: None,
                complete_sem,
                is_recursive: false,
                stopping: Arc::new(AtomicBool::new(true)),
            },
            action_tx,
        });
        let mut overlapped = Box::new(unsafe { std::mem::zeroed::<OVERLAPPED>() });
        overlapped.hEvent = Box::into_raw(request) as _;

        unsafe {
            // CancelIo can leave an already-queued completion with ERROR_SUCCESS. The invalid
            // handle makes any accidental attempt to rearm the request fail deterministically.
            handle_event(ERROR_SUCCESS, 0, Box::into_raw(overlapped));
            assert_eq!(
                WaitForSingleObjectEx(complete_sem, 0, 0),
                WAIT_OBJECT_0,
                "completion callback did not release the watch semaphore"
            );
            CloseHandle(complete_sem);
        }

        assert!(event_rx.try_iter().next().is_none(), "unexpected event");
        assert!(action_rx.try_iter().next().is_none(), "unexpected action");
    }

    #[test]
    fn watcher_drop_waits_for_the_native_server_callback_to_finish() {
        let root = tempfile::tempdir().expect("temporary watch root");
        let callback_gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
        let callback_gate_state = Arc::clone(&callback_gate);
        let mut watcher = ReadDirectoryChangesWatcher::new(
            move |_| {
                let (lock, condition) = &*callback_gate_state;
                let mut state = lock.lock().expect("callback gate");
                state.0 = true;
                condition.notify_all();
                while !state.1 {
                    state = condition.wait(state).expect("callback release");
                }
            },
            crate::Config::default(),
        )
        .expect("create watcher");
        watcher
            .watch(root.path(), RecursiveMode::Recursive)
            .expect("watch temporary root");
        std::fs::write(root.path().join("trigger.jpg"), b"trigger").expect("trigger callback");

        let (lock, condition) = &*callback_gate;
        let mut state = lock.lock().expect("main gate");
        while !state.0 {
            let (next_state, timeout) = condition
                .wait_timeout(state, Duration::from_secs(2))
                .expect("callback start");
            assert!(!timeout.timed_out(), "native callback did not start");
            state = next_state;
        }

        let (finished_tx, finished_rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            drop(watcher);
            let _ = finished_tx.send(());
        });
        assert!(
            finished_rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "watcher drop returned before its native callback completed"
        );

        state.1 = true;
        condition.notify_all();
        drop(state);
        finished_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("watcher server shutdown completion");
    }
}

/// Watcher implementation based on ReadDirectoryChanges
#[derive(Debug)]
pub struct ReadDirectoryChangesWatcher {
    tx: Sender<Action>,
    cmd_rx: Receiver<Result<PathBuf>>,
    wakeup_sem: HANDLE,
    server_thread: Option<JoinHandle<()>>,
}

impl ReadDirectoryChangesWatcher {
    pub fn create(
        event_handler: Arc<Mutex<dyn EventHandler>>,
        meta_tx: Sender<MetaEvent>,
    ) -> Result<ReadDirectoryChangesWatcher> {
        let (cmd_tx, cmd_rx) = unbounded();

        let wakeup_sem = unsafe { CreateSemaphoreW(ptr::null_mut(), 0, 1, ptr::null_mut()) };
        if wakeup_sem.is_null() || wakeup_sem == INVALID_HANDLE_VALUE {
            return Err(Error::generic("Failed to create wakeup semaphore."));
        }

        let (action_tx, server_thread) =
            match ReadDirectoryChangesServer::start(event_handler, meta_tx, cmd_tx, wakeup_sem) {
                Ok(server) => server,
                Err(error) => {
                    unsafe {
                        CloseHandle(wakeup_sem);
                    }
                    return Err(error);
                }
            };

        Ok(ReadDirectoryChangesWatcher {
            tx: action_tx,
            cmd_rx,
            wakeup_sem,
            server_thread: Some(server_thread),
        })
    }

    fn wakeup_server(&mut self) {
        // breaks the server out of its wait state.  right now this is really just an optimization,
        // so that if you add a watch you don't block for 100ms in watch() while the
        // server sleeps.
        unsafe {
            ReleaseSemaphore(self.wakeup_sem, 1, ptr::null_mut());
        }
    }

    fn shutdown_inner(&mut self) -> Result<()> {
        let Some(server_thread) = self.server_thread.take() else {
            return Ok(());
        };
        let send_result = self
            .tx
            .send(Action::Stop)
            .map_err(|_| Error::generic("Error stopping internal watcher thread"));
        if send_result.is_ok() {
            self.wakeup_server();
        }
        let join_result = server_thread
            .join()
            .map_err(|_| Error::generic("Internal watcher thread panicked during shutdown"));
        unsafe {
            CloseHandle(self.wakeup_sem);
        }
        send_result.and(join_result)
    }

    /// Stops the native server and waits until every watched handle has closed.
    pub fn shutdown(&mut self) -> Result<()> {
        self.shutdown_inner()
    }

    fn send_action_require_ack(&mut self, action: Action, pb: &PathBuf) -> Result<()> {
        self.tx
            .send(action)
            .map_err(|_| Error::generic("Error sending to internal channel"))?;

        // wake 'em up, we don't want to wait around for the ack
        self.wakeup_server();

        let ack_pb = self
            .cmd_rx
            .recv()
            .map_err(|_| Error::generic("Error receiving from command channel"))?
            .map_err(|e| Error::generic(&format!("Error in watcher: {:?}", e)))?;

        if pb.as_path() != ack_pb.as_path() {
            Err(Error::generic(&format!(
                "Expected ack for {:?} but got \
                 ack for {:?}",
                pb, ack_pb
            )))
        } else {
            Ok(())
        }
    }

    fn watch_inner(&mut self, path: &Path, recursive_mode: RecursiveMode) -> Result<()> {
        let pb = if path.is_absolute() {
            path.to_owned()
        } else {
            let p = env::current_dir().map_err(Error::io)?;
            p.join(path)
        };
        // path must exist and be either a file or directory
        if !pb.is_dir() && !pb.is_file() {
            return Err(Error::generic(
                "Input watch path is neither a file nor a directory.",
            ));
        }
        self.send_action_require_ack(Action::Watch(pb.clone(), recursive_mode), &pb)
    }

    fn unwatch_inner(&mut self, path: &Path) -> Result<()> {
        let pb = if path.is_absolute() {
            path.to_owned()
        } else {
            let p = env::current_dir().map_err(Error::io)?;
            p.join(path)
        };
        let res = self
            .tx
            .send(Action::Unwatch(pb))
            .map_err(|_| Error::generic("Error sending to internal channel"));
        self.wakeup_server();
        res
    }
}

impl Watcher for ReadDirectoryChangesWatcher {
    fn new<F: EventHandler>(event_handler: F, _config: Config) -> Result<Self> {
        // create dummy channel for meta event
        // TODO: determine the original purpose of this - can we remove it?
        let (meta_tx, _) = unbounded();
        let event_handler = Arc::new(Mutex::new(event_handler));
        Self::create(event_handler, meta_tx)
    }

    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> Result<()> {
        self.watch_inner(path, recursive_mode)
    }

    fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.unwatch_inner(path)
    }

    fn configure(&mut self, config: Config) -> Result<bool> {
        let (tx, rx) = bounded(1);
        self.tx.send(Action::Configure(config, tx))?;
        rx.recv()?
    }

    fn kind() -> crate::WatcherKind {
        WatcherKind::ReadDirectoryChangesWatcher
    }
}

impl Drop for ReadDirectoryChangesWatcher {
    fn drop(&mut self) {
        let _ = self.shutdown_inner();
    }
}

// `ReadDirectoryChangesWatcher` is not Send/Sync because of the semaphore Handle.
// As said elsewhere it's perfectly safe to send it across threads.
unsafe impl Send for ReadDirectoryChangesWatcher {}
// Because all public methods are `&mut self` it's also perfectly safe to share references.
unsafe impl Sync for ReadDirectoryChangesWatcher {}
