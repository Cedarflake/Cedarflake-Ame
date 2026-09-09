#[cfg(all(windows, test))]
use std::cell::{Cell, RefCell};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(windows)]
use std::collections::{HashSet, VecDeque};
use std::fs::{self, File, Metadata, ReadDir};
#[cfg(windows)]
use std::path::Prefix;
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::UNIX_EPOCH;

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};

#[cfg(windows)]
use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
#[cfg(windows)]
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_FOR_BACKUP_INTENT,
    FILE_OPEN_NO_RECALL as FILE_OPEN_NO_RECALL_NATIVE, FILE_OPEN_REPARSE_POINT,
    FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile as NtCreateFileRaw,
};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_NO_MORE_FILES, ERROR_REPARSE_POINT_ENCOUNTERED, HANDLE,
    INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, RtlNtStatusToDosError,
    UNICODE_STRING,
};
#[cfg(windows)]
use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
#[cfg(windows)]
use windows_sys::Win32::Storage::CloudFilters::{
    CF_PLACEHOLDER_STATE_INVALID, CF_PLACEHOLDER_STATE_PARTIAL,
    CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK, CF_PLACEHOLDER_STATE_PLACEHOLDER,
    CfGetPlaceholderStateFromAttributeTag,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
    FILE_BASIC_INFO, FILE_CASE_SENSITIVE_INFO, FILE_DELETE_CHILD, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_EXTD_DIR_INFO, FILE_ID_INFO,
    FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_TRAVERSE, FileAttributeTagInfo, FileBasicInfo, FileCaseSensitiveInfo,
    FileIdExtdDirectoryInfo, FileIdExtdDirectoryRestartInfo, FileIdInfo, FindClose, FindFirstFileW,
    GetFileInformationByHandleEx, GetFinalPathNameByHandleW, GetLongPathNameW,
    GetVolumeInformationByHandleW, SYNCHRONIZE, WIN32_FIND_DATAW, WRITE_DAC, WRITE_OWNER,
};
#[cfg(all(windows, test))]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_READ_EA, FILE_WRITE_ATTRIBUTES, READ_CONTROL, SetFileInformationByHandle,
};
#[cfg(windows)]
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
#[cfg(windows)]
use windows_sys::Win32::System::SystemServices::FILE_CS_FLAG_CASE_SENSITIVE_DIR;

use crate::domain::{
    DiscoveredFile, ExpectedFileState, FileIdentityEvidence, LibraryRootAvailability,
    MediaInspection, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, RootAvailabilityEvidence, ScanError, ScanIssue,
    SourceRevisionEvidence,
};

mod catalog_identity;
mod file_admission;
mod media_signature;
mod preview_cache_namespace;
#[cfg(windows)]
mod viewer_source_guard;
pub(crate) use catalog_identity::{open_catalog_identity_guard, read_catalog_identity};
pub use file_admission::{FileVisit, FileVisitOutcome};
pub(crate) use preview_cache_namespace::PreviewCacheNamespace;
#[cfg(windows)]
pub(crate) use viewer_source_guard::open_viewer_source_guard;

#[cfg(windows)]
const HANDLE_DIRECTORY_BUFFER_BYTES: usize = 64 * 1024;

#[cfg(windows)]
const WINDOWS_TO_UNIX_EPOCH_100NS: i64 = 116_444_736_000_000_000;

#[cfg(windows)]
const HUNDRED_NS_PER_MILLISECOND: i64 = 10_000;

#[cfg(windows)]
const WINDOWS_SOURCE_REVISION_SCHEME: &str = "windows-file-change-time-100ns-v1";

#[cfg(test)]
static SOURCE_ENUMERATION_COUNTS: LazyLock<Mutex<HashMap<String, SourceEnumerationCounts>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(all(windows, test))]
static CONFIGURED_ROOT_OPEN_COUNTS: LazyLock<Mutex<HashMap<(PathBuf, bool), u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
static ROOT_AVAILABILITY_METADATA_PROBE_COUNTS: LazyLock<Mutex<HashMap<PathBuf, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(all(windows, test))]
static SOURCE_CONTENT_OPEN_COUNTS: LazyLock<Mutex<HashMap<PathBuf, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
#[derive(Default)]
struct SourceEnumerationCounts {
    entry_reads: u64,
    directory_opens: u64,
    peak_staged_window: usize,
}

#[cfg(test)]
static SOURCE_ENUMERATION_GATES: LazyLock<Mutex<HashMap<String, Arc<SourceEnumerationGateState>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(all(windows, test))]
thread_local! {
    static PINNED_SOURCE_ACCESS_UPGRADE_COUNT: Cell<u64> = const { Cell::new(0) };
    static PINNED_SOURCE_DATA_AUTHORIZATION_COUNT: Cell<u64> = const { Cell::new(0) };
    static LAST_ROOT_RELATIVE_METADATA_OPEN: Cell<Option<RootRelativeNtOpenFacts>> = const { Cell::new(None) };
    static LAST_ROOT_RELATIVE_DATA_OPEN: Cell<Option<RootRelativeNtOpenFacts>> = const { Cell::new(None) };
    static CONFIGURED_ROOT_OPENS: RefCell<Vec<ConfiguredRootOpenFacts>> = const { RefCell::new(Vec::new()) };
    static PUBLICATION_NAMESPACE_GUARD_OPENS: RefCell<Vec<PublicationNamespaceGuardOpenFacts>> = const { RefCell::new(Vec::new()) };
    static FORCE_PUBLICATION_NAMESPACE_GUARD_FAILURE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(all(windows, test))]
pub(crate) struct ForcedPublicationNamespaceGuardFailure;

#[cfg(all(windows, test))]
impl Drop for ForcedPublicationNamespaceGuardFailure {
    fn drop(&mut self) {
        FORCE_PUBLICATION_NAMESPACE_GUARD_FAILURE.with(|forced| forced.set(false));
    }
}

#[cfg(all(windows, test))]
pub(crate) fn force_publication_namespace_guard_failure_for_test()
-> ForcedPublicationNamespaceGuardFailure {
    FORCE_PUBLICATION_NAMESPACE_GUARD_FAILURE.with(|forced| {
        assert!(
            !forced.replace(true),
            "publication guard failure already forced"
        );
    });
    ForcedPublicationNamespaceGuardFailure
}

#[cfg(all(windows, test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootRelativeNtOpenFacts {
    desired_access: u32,
    share_access: u32,
    create_options: u32,
    ea_buffer_is_null: bool,
    ea_length: u32,
    object_name_is_absolute: bool,
    object_name_has_separator: bool,
}

#[cfg(all(windows, test))]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfiguredRootOpenFacts {
    path: PathBuf,
    desired_access: u32,
    share_access: u32,
    flags: u32,
}

#[cfg(all(windows, test))]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PublicationNamespaceGuardOpenFacts {
    path: PathBuf,
    desired_access: u32,
    share_access: u32,
    flags: u32,
}

#[cfg(test)]
struct SourceEnumerationGateState {
    control: Mutex<SourceEnumerationGateControl>,
    changed: Condvar,
}

#[cfg(test)]
struct SourceEnumerationGateControl {
    permits: usize,
    waiting: usize,
    released: bool,
}

#[cfg(test)]
pub(crate) struct SourceEnumerationGateGuard {
    root_path: String,
    state: Arc<SourceEnumerationGateState>,
}

#[cfg(test)]
impl SourceEnumerationGateGuard {
    pub(crate) fn allow_entries(&self, count: usize) {
        let mut control = self.state.control.lock().expect("source enumeration gate");
        control.permits = control.permits.saturating_add(count);
        self.state.changed.notify_all();
    }

    pub(crate) fn wait_until_blocked(&self, timeout: std::time::Duration) -> bool {
        let control = self.state.control.lock().expect("source enumeration gate");
        let (control, _) = self
            .state
            .changed
            .wait_timeout_while(control, timeout, |control| {
                control.waiting == 0 && !control.released
            })
            .expect("source enumeration gate wait");
        control.waiting > 0
    }

    pub(crate) fn release(&self) {
        let mut control = self.state.control.lock().expect("source enumeration gate");
        control.released = true;
        self.state.changed.notify_all();
    }
}

#[cfg(test)]
impl Drop for SourceEnumerationGateGuard {
    fn drop(&mut self) {
        self.release();
        let mut gates = SOURCE_ENUMERATION_GATES
            .lock()
            .expect("source enumeration gates");
        if gates
            .get(&self.root_path)
            .is_some_and(|state| Arc::ptr_eq(state, &self.state))
        {
            gates.remove(&self.root_path);
        }
    }
}

#[cfg(test)]
pub(crate) fn gate_source_enumeration(root_path: &str) -> SourceEnumerationGateGuard {
    let state = Arc::new(SourceEnumerationGateState {
        control: Mutex::new(SourceEnumerationGateControl {
            permits: 0,
            waiting: 0,
            released: false,
        }),
        changed: Condvar::new(),
    });
    SOURCE_ENUMERATION_GATES
        .lock()
        .expect("source enumeration gates")
        .insert(root_path.to_owned(), Arc::clone(&state));
    SourceEnumerationGateGuard {
        root_path: root_path.to_owned(),
        state,
    }
}

#[cfg(test)]
fn wait_for_source_enumeration_permit(root_path: &str) {
    let state = SOURCE_ENUMERATION_GATES
        .lock()
        .expect("source enumeration gates")
        .get(root_path)
        .cloned();
    let Some(state) = state else {
        return;
    };
    let mut control = state.control.lock().expect("source enumeration gate");
    control.waiting = control.waiting.saturating_add(1);
    state.changed.notify_all();
    while control.permits == 0 && !control.released {
        control = state
            .changed
            .wait(control)
            .expect("source enumeration gate wait");
    }
    control.waiting = control.waiting.saturating_sub(1);
    if !control.released {
        control.permits = control.permits.saturating_sub(1);
    }
}

#[cfg(test)]
pub(crate) fn reset_source_enumeration_instrumentation(root_path: &str) {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .insert(root_path.to_owned(), SourceEnumerationCounts::default());
}

#[cfg(all(windows, test))]
pub(crate) fn reset_configured_root_open_instrumentation(root_path: &str) {
    let root = PathBuf::from(root_path);
    CONFIGURED_ROOT_OPEN_COUNTS
        .lock()
        .expect("configured-root open instrumentation")
        .retain(|(path, _), _| path != &root);
}

#[cfg(all(windows, test))]
pub(crate) fn configured_root_open_count(root_path: &str, share_root_delete: bool) -> u64 {
    CONFIGURED_ROOT_OPEN_COUNTS
        .lock()
        .expect("configured-root open instrumentation")
        .get(&(PathBuf::from(root_path), share_root_delete))
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn reset_root_availability_metadata_probe_instrumentation(root_path: &str) {
    ROOT_AVAILABILITY_METADATA_PROBE_COUNTS
        .lock()
        .expect("root availability metadata-probe instrumentation")
        .insert(PathBuf::from(root_path), 0);
}

#[cfg(test)]
pub(crate) fn root_availability_metadata_probe_count(root_path: &str) -> u64 {
    ROOT_AVAILABILITY_METADATA_PROBE_COUNTS
        .lock()
        .expect("root availability metadata-probe instrumentation")
        .get(&PathBuf::from(root_path))
        .copied()
        .unwrap_or(0)
}

#[cfg(all(windows, test))]
pub(crate) fn reset_source_content_open_instrumentation(root_path: &str) {
    let root =
        std::fs::canonicalize(root_path).expect("instrumented source root is canonicalizable");
    SOURCE_CONTENT_OPEN_COUNTS
        .lock()
        .expect("source content open instrumentation")
        .insert(root, 0);
}

#[cfg(all(windows, test))]
pub(crate) fn source_content_open_count(root_path: &str) -> u64 {
    let root =
        std::fs::canonicalize(root_path).expect("instrumented source root is canonicalizable");
    SOURCE_CONTENT_OPEN_COUNTS
        .lock()
        .expect("source content open instrumentation")
        .get(&root)
        .copied()
        .unwrap_or(0)
}

#[cfg(all(windows, test))]
fn record_source_content_open(root_path: &Path) {
    let root = std::fs::canonicalize(root_path).expect("opened source root is canonicalizable");
    let mut counts = SOURCE_CONTENT_OPEN_COUNTS
        .lock()
        .expect("source content open instrumentation");
    if let Some(count) = counts.get_mut(&root) {
        *count = count.saturating_add(1);
    }
}

#[cfg(test)]
pub(crate) fn source_entry_read_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.entry_reads)
}

#[cfg(test)]
pub(crate) fn source_directory_open_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.directory_opens)
}

#[cfg(test)]
pub(crate) fn source_peak_staged_window(root_path: &str) -> usize {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.peak_staged_window)
}

#[cfg(test)]
pub(crate) fn record_source_peak_staged_window(root_path: &str, staged: usize) {
    if let Some(counts) = SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get_mut(root_path)
    {
        counts.peak_staged_window = counts.peak_staged_window.max(staged);
    }
}

pub struct FileDiscovery {
    root: PathBuf,
    canonical_root: PathBuf,
    #[cfg(windows)]
    root_proof: WindowsDirectoryRootProof,
    #[cfg(windows)]
    _publication_namespace_ancestors: Vec<File>,
}

pub(crate) struct PublicationGuardedFileDiscovery {
    discovery: FileDiscovery,
}

#[cfg(windows)]
struct WindowsDirectoryRootProof {
    handle: File,
    metadata_handle: Option<File>,
    identity: FileIdentityEvidence,
    volume_serial: u64,
}

enum RootAvailabilityMetadataEvidence {
    AvailableDirectory,
    OfflineDirectory,
    NotDirectory,
    Missing,
    PermissionDenied,
    Inaccessible(String),
}

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    classify_root_availability_metadata(probe_root_availability_metadata(root_path))
}

fn probe_root_availability_metadata(root_path: &str) -> RootAvailabilityMetadataEvidence {
    let path = std::path::Path::new(root_path);
    #[cfg(test)]
    {
        let mut counts = ROOT_AVAILABILITY_METADATA_PROBE_COUNTS
            .lock()
            .expect("root availability metadata-probe instrumentation");
        let count = counts.entry(path.to_path_buf()).or_insert(0);
        *count = count.saturating_add(1);
    }
    match path.symlink_metadata() {
        Ok(metadata) => match entry_reparse_evidence(path, &metadata) {
            Ok((_, state)) if state != MetadataInventoryPlaceholderState::Available => {
                RootAvailabilityMetadataEvidence::OfflineDirectory
            }
            Ok((_, _)) if metadata.is_dir() => RootAvailabilityMetadataEvidence::AvailableDirectory,
            Ok(_) => RootAvailabilityMetadataEvidence::NotDirectory,
            Err(error) => {
                let mut message = "The source root cannot be classified safely: ".to_owned();
                message.push_str(&error.to_string());
                RootAvailabilityMetadataEvidence::Inaccessible(message)
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            RootAvailabilityMetadataEvidence::Missing
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            RootAvailabilityMetadataEvidence::PermissionDenied
        }
        Err(error) => {
            let mut message = "The source root is unavailable: ".to_owned();
            message.push_str(&error.to_string());
            RootAvailabilityMetadataEvidence::Inaccessible(message)
        }
    }
}

fn classify_root_availability_metadata(
    evidence: RootAvailabilityMetadataEvidence,
) -> RootAvailabilityEvidence {
    match evidence {
        RootAvailabilityMetadataEvidence::AvailableDirectory => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Available,
            message: None,
        },
        RootAvailabilityMetadataEvidence::OfflineDirectory => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Offline,
            message: Some("The source root is not locally available".to_owned()),
        },
        RootAvailabilityMetadataEvidence::NotDirectory => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Missing,
            message: Some("The stored source path is no longer a directory".to_owned()),
        },
        RootAvailabilityMetadataEvidence::Missing => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Missing,
            message: Some("The source root is missing or its volume is disconnected".to_owned()),
        },
        RootAvailabilityMetadataEvidence::PermissionDenied => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Inaccessible,
            message: Some("The source root cannot be accessed with current permissions".to_owned()),
        },
        RootAvailabilityMetadataEvidence::Inaccessible(message) => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Inaccessible,
            message: Some(message),
        },
    }
}

pub struct DirectoryEntryPaths {
    root: PathBuf,
    directory_path: PathBuf,
    entries: ReadDir,
    index: u64,
}

pub(crate) struct CheckedDirectoryEntryPaths {
    entries: Vec<CheckedDirectoryEntryPath>,
    next_index: usize,
    #[cfg(test)]
    directory_identity: Option<FileIdentityEvidence>,
}

struct StreamingCheckedDirectoryEntryPaths {
    #[cfg(not(windows))]
    root: PathBuf,
    #[cfg(not(windows))]
    directory_path: PathBuf,
    #[cfg(not(windows))]
    entries: ReadDir,
    #[cfg(windows)]
    directory: File,
    #[cfg(windows)]
    directory_relative_path: String,
    #[cfg(windows)]
    buffer: Box<[u64]>,
    #[cfg(windows)]
    pending_entries: VecDeque<Result<MetadataInventoryEntry, ScanIssue>>,
    #[cfg(windows)]
    observed_paths: HashSet<String>,
    #[cfg(windows)]
    volume_serial: u64,
    #[cfg(windows)]
    should_restart: bool,
    #[cfg(windows)]
    enumeration_complete: bool,
    #[cfg(windows)]
    enumeration_failed: bool,
    directory_identity: Option<FileIdentityEvidence>,
    #[cfg(test)]
    test_counter_root: Option<String>,
}

struct CheckedDirectoryEntryPath {
    relative_path: String,
    path: PathBuf,
}

pub(crate) struct PublicationGuardedFileVisits<'guard> {
    discovery: &'guard FileDiscovery,
    entries: CheckedDirectoryEntryPaths,
}

pub(crate) struct PublicationGuardedMetadataInventoryEntries {
    entries: StreamingCheckedDirectoryEntryPaths,
    _guard: PublicationGuardedFileDiscovery,
}

pub(crate) struct CheckedDirectoryEntry {
    relative_path: String,
    metadata: Metadata,
    reparse_kind: ReparseKind,
    placeholder_state: MetadataInventoryPlaceholderState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReparseKind {
    None,
    CloudFiles,
    Other,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttributeTagEvidence {
    attributes: u32,
    reparse_tag: u32,
}

#[cfg(windows)]
#[derive(Debug)]
struct RootRelativeContainmentViolation;

#[cfg(windows)]
impl std::fmt::Display for RootRelativeContainmentViolation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The root-relative source path crossed a reparse point or changed type")
    }
}

#[cfg(windows)]
impl std::error::Error for RootRelativeContainmentViolation {}

impl Iterator for DirectoryEntryPaths {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.next()?;
        let relative_path = match entry {
            Ok(entry) => entry
                .path()
                .strip_prefix(&self.root)
                .map(relative_path_text)
                .unwrap_or_else(|_| path_text(entry.path())),
            Err(_) => {
                let unresolved = self
                    .directory_path
                    .join(format!("<unresolved-entry-{}>", self.index));
                unresolved
                    .strip_prefix(&self.root)
                    .map(relative_path_text)
                    .unwrap_or_else(|_| path_text(unresolved))
            }
        };
        self.index = self.index.saturating_add(1);
        Some(relative_path)
    }
}

impl Iterator for CheckedDirectoryEntryPaths {
    type Item = Result<CheckedDirectoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.get(self.next_index)?;
        self.next_index = self.next_index.saturating_add(1);
        Some(
            fs::symlink_metadata(&entry.path)
                .map_err(|error| ScanIssue {
                    path: Some(path_text(&entry.path)),
                    code: "directory_entry_metadata_unreadable".to_owned(),
                    message: error.to_string(),
                })
                .and_then(|metadata| {
                    checked_directory_entry_from_metadata(
                        entry.relative_path.clone(),
                        &entry.path,
                        metadata,
                    )
                }),
        )
    }
}

impl Iterator for PublicationGuardedFileVisits<'_> {
    type Item = Result<FileVisit, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| entry.map(|entry| self.discovery.visit_directory_entry(entry)))
    }
}

impl Iterator for PublicationGuardedMetadataInventoryEntries {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next()
    }
}

impl PublicationGuardedMetadataInventoryEntries {
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.entries.directory_identity()
    }

    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        self.entries.finish()
    }
}

#[cfg(not(windows))]
impl Iterator for StreamingCheckedDirectoryEntryPaths {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref() {
            wait_for_source_enumeration_permit(root_path);
        }
        let entry = self.entries.next()?;
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref()
            && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(root_path)
        {
            counts.entry_reads = counts.entry_reads.saturating_add(1);
        }
        Some(
            entry
                .map_err(|error| ScanIssue {
                    path: Some(path_text(&self.directory_path)),
                    code: "directory_entry_unreadable".to_owned(),
                    message: error.to_string(),
                })
                .and_then(|entry| {
                    let path = entry.path();
                    let relative_path = path
                        .strip_prefix(&self.root)
                        .map(relative_path_text)
                        .map_err(|_| path_containment_issue(&path))?;
                    fs::symlink_metadata(&path)
                        .map_err(|error| ScanIssue {
                            path: Some(path_text(&path)),
                            code: "directory_entry_metadata_unreadable".to_owned(),
                            message: error.to_string(),
                        })
                        .and_then(|metadata| {
                            checked_directory_entry_from_metadata(relative_path, &path, metadata)
                        })
                        .and_then(|entry| {
                            metadata_inventory_entry_from_checked_directory_entry(entry, &path)
                        })
                }),
        )
    }
}

#[cfg(windows)]
impl Iterator for StreamingCheckedDirectoryEntryPaths {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref() {
            wait_for_source_enumeration_permit(root_path);
        }
        if let Some(entry) = self.pending_entries.pop_front() {
            self.record_entry_read();
            return Some(entry);
        }
        if self.enumeration_complete || self.enumeration_failed {
            return None;
        }
        let mut empty_nonterminal_pages = 0_u8;
        loop {
            match self.read_next_handle_page() {
                Ok(()) => {
                    if let Some(entry) = self.pending_entries.pop_front() {
                        self.record_entry_read();
                        return Some(entry);
                    }
                    if self.enumeration_complete {
                        return None;
                    }
                    empty_nonterminal_pages = empty_nonterminal_pages.saturating_add(1);
                    if empty_nonterminal_pages >= 2 {
                        self.enumeration_failed = true;
                        return Some(Err(directory_enumeration_issue(
                            &self.directory_relative_path,
                            "The directory handle returned repeated empty non-terminal pages",
                        )));
                    }
                }
                Err(issue) => {
                    self.enumeration_failed = true;
                    return Some(Err(issue));
                }
            }
        }
    }
}

#[cfg(windows)]
impl StreamingCheckedDirectoryEntryPaths {
    fn record_entry_read(&self) {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref()
            && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(root_path)
        {
            counts.entry_reads = counts.entry_reads.saturating_add(1);
        }
    }

    fn read_next_handle_page(&mut self) -> Result<(), ScanIssue> {
        self.buffer.fill(0);
        let information_class = if self.should_restart {
            FileIdExtdDirectoryRestartInfo
        } else {
            FileIdExtdDirectoryInfo
        };
        let buffer_size = u32::try_from(self.buffer.len().saturating_mul(size_of::<u64>()))
            .map_err(|_| {
                directory_enumeration_issue(
                    &self.directory_relative_path,
                    "The directory enumeration buffer is too large",
                )
            })?;
        // SAFETY: ADR 0024 owns this adapter-only boundary. `directory` remains live and was
        // opened no-follow with FILE_LIST_DIRECTORY; `buffer` is aligned, initialized, writable,
        // and exactly `buffer_size` bytes long for the duration of the call.
        let result = unsafe {
            GetFileInformationByHandleEx(
                self.directory.as_raw_handle(),
                information_class,
                self.buffer.as_mut_ptr().cast(),
                buffer_size,
            )
        };
        if result == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                self.enumeration_complete = true;
                return Ok(());
            }
            return Err(ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            });
        }
        self.should_restart = false;
        let byte_len = self.buffer.len().saturating_mul(size_of::<u64>());
        // SAFETY: the initialized `u64` allocation is exposed only as immutable bytes for the
        // safe, bounds-checked parser; the allocation remains alive and is not mutated meanwhile.
        let bytes =
            unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), byte_len) };
        let entries = parse_handle_directory_buffer(
            bytes,
            self.volume_serial,
            &self.directory_relative_path,
        )?;
        for entry in entries {
            if !self.observed_paths.insert(entry.relative_path.clone()) {
                return Err(directory_enumeration_issue(
                    &self.directory_relative_path,
                    "The directory handle returned a duplicate relative path",
                ));
            }
            self.pending_entries.push_back(Ok(entry));
        }
        Ok(())
    }
}

impl StreamingCheckedDirectoryEntryPaths {
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.directory_identity.as_ref()
    }

    #[cfg(not(windows))]
    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        let closing_metadata = self
            .directory_path
            .symlink_metadata()
            .map_err(|error| path_metadata_issue(&self.directory_path, error))?;
        if !closing_metadata.is_dir()
            || (self.directory_path != self.root && is_link_or_reparse_point(&closing_metadata))
        {
            return Err(path_containment_issue(&self.directory_path));
        }
        let closing_identity = file_identity(&self.directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&self.directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if closing_identity != self.directory_identity {
            return Err(ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its durable spool was captured"
                    .to_owned(),
            });
        }
        Ok(closing_identity)
    }

    #[cfg(windows)]
    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        if self.enumeration_failed || !self.enumeration_complete || !self.pending_entries.is_empty()
        {
            return Err(directory_enumeration_issue(
                &self.directory_relative_path,
                "The directory handle enumeration did not reach a trustworthy terminal boundary",
            ));
        }
        let closing_identity =
            file_identity_from_handle(&self.directory).map_err(|error| ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
        if closing_identity != self.directory_identity {
            return Err(ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its durable spool was captured"
                    .to_owned(),
            });
        }
        Ok(closing_identity)
    }
}

impl CheckedDirectoryEntryPaths {
    #[cfg(test)]
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.directory_identity.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn seek_after(&mut self, relative_path: &str) {
        self.next_index = self
            .entries
            .partition_point(|entry| entry.relative_path.as_str() <= relative_path);
    }
}

fn stable_checked_directory_entries(
    root: &Path,
    directory_path: &Path,
    entries: ReadDir,
) -> Result<Vec<CheckedDirectoryEntryPath>, ScanIssue> {
    let mut stable_entries = entries
        .map(|entry| {
            let entry = entry.map_err(|error| ScanIssue {
                path: Some(path_text(directory_path)),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(root)
                .map(relative_path_text)
                .map_err(|_| path_containment_issue(&path))?;
            Ok(CheckedDirectoryEntryPath {
                relative_path,
                path,
            })
        })
        .collect::<Result<Vec<_>, ScanIssue>>()?;
    stable_entries.sort_unstable_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if stable_entries
        .windows(2)
        .any(|entries| entries[0].relative_path == entries[1].relative_path)
    {
        return Err(ScanIssue {
            path: Some(path_text(directory_path)),
            code: "directory_entry_identity_ambiguous".to_owned(),
            message: "The directory contains paths that cannot be ordered uniquely".to_owned(),
        });
    }
    Ok(stable_entries)
}

#[cfg(windows)]
fn parse_handle_directory_buffer(
    buffer: &[u8],
    volume_serial: u64,
    relative_directory: &str,
) -> Result<Vec<MetadataInventoryEntry>, ScanIssue> {
    let header_size = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
    let mut offset = 0_usize;
    let mut entries = Vec::new();
    loop {
        let record = buffer.get(offset..).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid record offset",
            )
        })?;
        if record.len() < header_size {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a truncated record header",
            ));
        }
        let next_offset = read_u32_field(
            record,
            std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
        )? as usize;
        let name_length = read_u32_field(
            record,
            std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileNameLength),
        )? as usize;
        if name_length == 0 || !name_length.is_multiple_of(size_of::<u16>()) {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid UTF-16 name length",
            ));
        }
        let record_extent = if next_offset == 0 {
            record.len()
        } else {
            if next_offset < header_size
                || next_offset > record.len()
                || !next_offset.is_multiple_of(std::mem::align_of::<FILE_ID_EXTD_DIR_INFO>())
            {
                return Err(directory_enumeration_issue(
                    relative_directory,
                    "The directory handle returned an invalid continuation offset",
                ));
            }
            next_offset
        };
        let name_end = header_size.checked_add(name_length).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle name length overflowed",
            )
        })?;
        if name_end > record_extent {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a truncated UTF-16 name",
            ));
        }
        let name_bytes = &record[header_size..name_end];
        let name_units = name_bytes
            .chunks_exact(size_of::<u16>())
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        let name = String::from_utf16(&name_units).map_err(|_| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid UTF-16 name",
            )
        })?;
        if name.contains(['\0', '/', '\\']) {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a name that is not one path component",
            ));
        }
        if name != "." && name != ".." {
            let attributes = read_u32_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileAttributes),
            )?;
            let reparse_tag = read_u32_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ReparsePointTag),
            )?;
            let end_of_file = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, EndOfFile),
            )?;
            let modified_100ns = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, LastWriteTime),
            )?;
            let change_time_100ns = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ChangeTime),
            )?;
            let file_id_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileId);
            let file_id_bytes: [u8; 16] = record
                .get(file_id_offset..file_id_offset + 16)
                .and_then(|value| value.try_into().ok())
                .ok_or_else(|| {
                    directory_enumeration_issue(
                        relative_directory,
                        "The directory handle returned a truncated file identity",
                    )
                })?;
            entries.push(metadata_inventory_entry_from_handle_record(
                relative_directory,
                &name,
                attributes,
                reparse_tag,
                end_of_file,
                modified_100ns,
                change_time_100ns,
                volume_serial,
                file_id_bytes,
            )?);
        }
        if next_offset == 0 {
            break;
        }
        offset = offset.checked_add(next_offset).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle continuation offset overflowed",
            )
        })?;
    }
    Ok(entries)
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn metadata_inventory_entry_from_handle_record(
    relative_directory: &str,
    name: &str,
    attributes: u32,
    reparse_tag: u32,
    end_of_file: i64,
    modified_100ns: i64,
    change_time_100ns: i64,
    volume_serial: u64,
    file_id_bytes: [u8; 16],
) -> Result<MetadataInventoryEntry, ScanIssue> {
    let relative_path = if relative_directory.is_empty() {
        name.to_owned()
    } else {
        format!("{relative_directory}/{name}")
    };
    let (reparse_kind, placeholder_state) = if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        (
            ReparseKind::None,
            metadata_placeholder_state_from_attributes(attributes),
        )
    } else {
        reparse_evidence_from_attribute_tag(attributes, reparse_tag).map_err(|error| ScanIssue {
            path: Some(relative_path.clone()),
            code: "file_reparse_evidence_unreadable".to_owned(),
            message: error.to_string(),
        })?
    };
    let is_directory = attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    let is_reparse_point = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    let is_opaque_directory = is_directory
        && (is_reparse_point || placeholder_state != MetadataInventoryPlaceholderState::Available);
    let kind = if reparse_kind == ReparseKind::Other || is_opaque_directory {
        MetadataInventoryEntryKind::Other
    } else {
        metadata_inventory_entry_kind(is_directory, !is_directory, reparse_kind)
    };
    let file_size = if kind == MetadataInventoryEntryKind::File {
        Some(u64::try_from(end_of_file).map_err(|_| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a negative file size",
            )
        })?)
    } else {
        None
    };
    let file_identity = (kind == MetadataInventoryEntryKind::File
        && placeholder_state == MetadataInventoryPlaceholderState::Available)
        .then(|| FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".to_owned(),
            value: format!(
                "{volume_serial:016x}:{:032x}",
                u128::from_le_bytes(file_id_bytes)
            ),
        });
    Ok(MetadataInventoryEntry {
        relative_path,
        kind,
        file_size,
        modified_unix_ms: windows_file_time_to_unix_ms(modified_100ns),
        file_identity,
        source_revision: (kind == MetadataInventoryEntryKind::File
            && placeholder_state == MetadataInventoryPlaceholderState::Available)
            .then(|| windows_source_revision(change_time_100ns)),
        placeholder_state,
        is_reparse_point,
    })
}

#[cfg(windows)]
fn read_u32_field(record: &[u8], offset: usize) -> Result<u32, ScanIssue> {
    let value: [u8; 4] = record
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| {
            directory_enumeration_issue("", "The directory handle record is truncated")
        })?;
    Ok(u32::from_le_bytes(value))
}

#[cfg(windows)]
fn read_i64_field(record: &[u8], offset: usize) -> Result<i64, ScanIssue> {
    let value: [u8; 8] = record
        .get(offset..offset + 8)
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| {
            directory_enumeration_issue("", "The directory handle record is truncated")
        })?;
    Ok(i64::from_le_bytes(value))
}

#[cfg(windows)]
fn windows_file_time_to_unix_ms(value: i64) -> i64 {
    value
        .saturating_sub(WINDOWS_TO_UNIX_EPOCH_100NS)
        .checked_div(HUNDRED_NS_PER_MILLISECOND)
        .unwrap_or(0)
        .max(0)
}

#[cfg(windows)]
fn directory_enumeration_issue(relative_directory: &str, message: &str) -> ScanIssue {
    ScanIssue {
        path: (!relative_directory.is_empty()).then(|| relative_directory.to_owned()),
        code: "directory_entry_unreadable".to_owned(),
        message: message.to_owned(),
    }
}

impl FileDiscovery {
    pub fn new(root_path: &str) -> Result<Self, ScanError> {
        Self::new_with_root_delete_sharing(root_path, true)
    }

    fn new_with_root_delete_sharing(
        root_path: &str,
        share_root_delete: bool,
    ) -> Result<Self, ScanError> {
        let root = PathBuf::from(root_path);
        if !root.is_absolute() {
            return Err(ScanError::new(
                "root_not_absolute",
                "The library root must be an absolute path",
            ));
        }
        #[cfg(windows)]
        if windows_path_uses_device_namespace(&root) {
            return Err(ScanError::new(
                "root_device_namespace_unsupported",
                "Library roots must be filesystem paths, not raw volume or device namespaces",
            ));
        }
        #[cfg(not(windows))]
        if !root.is_dir() {
            return Err(ScanError::new(
                "root_unavailable",
                "The selected library root is not an available directory",
            ));
        }

        #[cfg(windows)]
        let (root_proof, canonical_root) = WindowsDirectoryRootProof::open_configured(
            &root,
            share_root_delete,
        )
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ScanError::new(
                    "root_unavailable",
                    "The selected library root is not an available directory",
                )
            } else {
                ScanError::new(
                    "root_identity_unavailable",
                    format!("Could not bind the selected directory to a safe handle: {error}"),
                )
            }
        })?;
        #[cfg(not(windows))]
        let canonical_root = canonical_source_root_path(&root).map_err(|error| {
            ScanError::new(
                "root_canonicalization_failed",
                format!("Could not resolve the selected directory: {error}"),
            )
        })?;

        Ok(Self {
            root,
            canonical_root,
            #[cfg(windows)]
            root_proof,
            #[cfg(windows)]
            _publication_namespace_ancestors: Vec::new(),
        })
    }
}

impl PublicationGuardedFileDiscovery {
    pub(crate) fn new_metadata_inventory_publication_guard(
        root_path: &str,
        expected_identity: &FileIdentityEvidence,
    ) -> Result<PublicationGuardedFileDiscovery, ScanError> {
        Self::new_metadata_inventory_source_guard(root_path, Some(expected_identity))
    }

    pub(crate) fn new_metadata_inventory_source_guard(
        root_path: &str,
        expected_publication_identity: Option<&FileIdentityEvidence>,
    ) -> Result<PublicationGuardedFileDiscovery, ScanError> {
        #[cfg(test)]
        if FORCE_PUBLICATION_NAMESPACE_GUARD_FAILURE.with(Cell::get) {
            return Err(ScanError::new(
                "root_publication_namespace_guard_unsupported",
                "The configured namespace does not grant the required publication-guard capability",
            ));
        }
        let root = PathBuf::from(root_path);
        if !root.is_absolute() || windows_path_uses_device_namespace(&root) {
            return Err(ScanError::new(
                "root_publication_namespace_invalid",
                "The publication namespace must be an absolute filesystem directory path",
            ));
        }
        let (root_proof, canonical_root, publication_namespace_ancestors) =
            WindowsDirectoryRootProof::open_publication_namespace(
                &root,
                expected_publication_identity,
            )
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::InvalidData {
                    ScanError::new(
                        "metadata_inventory_root_identity_changed",
                        "The configured root no longer matches its durable publication identity",
                    )
                } else if error.kind() == std::io::ErrorKind::PermissionDenied {
                    ScanError::new(
                        "root_publication_namespace_guard_unsupported",
                        format!(
                            "The configured namespace contains an ancestor that cannot be pinned without delete sharing: {error}"
                        ),
                    )
                } else {
                    ScanError::new(
                        "root_publication_namespace_unavailable",
                        format!("Could not pin the configured publication namespace: {error}"),
                    )
                }
            })?;
        let discovery = FileDiscovery {
            root,
            canonical_root,
            root_proof,
            _publication_namespace_ancestors: publication_namespace_ancestors,
        };
        if let Some(expected_identity) = expected_publication_identity {
            discovery.require_metadata_inventory_root_identity(expected_identity)?;
        }
        Ok(Self { discovery })
    }

    pub(crate) fn new_incremental_publication_guard(
        root_path: &str,
        expected_identity: &FileIdentityEvidence,
    ) -> Result<PublicationGuardedFileDiscovery, ScanError> {
        Self::new_metadata_inventory_publication_guard(root_path, expected_identity)
    }

    pub(crate) fn metadata_inventory_root_identity(
        &self,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        self.discovery.metadata_inventory_root_identity()
    }

    pub(crate) fn require_metadata_inventory_root_identity(
        &self,
        expected: &FileIdentityEvidence,
    ) -> Result<(), ScanError> {
        self.discovery
            .require_metadata_inventory_root_identity(expected)
    }

    pub(crate) fn metadata_inventory_entry(
        &self,
        relative_path: &str,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        self.discovery.metadata_inventory_entry(relative_path)
    }

    pub(crate) fn visit_relative_path(&self, relative_path: &str) -> FileVisit {
        self.discovery.visit_relative_path(relative_path)
    }

    pub(crate) fn file_visits_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<PublicationGuardedFileVisits<'_>, ScanIssue> {
        Ok(PublicationGuardedFileVisits {
            discovery: &self.discovery,
            entries: self
                .discovery
                .checked_entry_paths_in_directory(relative_directory)?,
        })
    }

    pub(crate) fn streaming_metadata_inventory_entries_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<PublicationGuardedMetadataInventoryEntries, ScanIssue> {
        let guard = self.try_clone_capability().map_err(|error| ScanIssue {
            path: Some(path_text(&self.discovery.canonical_root)),
            code: "root_publication_namespace_guard_clone_failed".to_owned(),
            message: format!("Could not retain the publication guard for enumeration: {error}"),
        })?;
        let entries = guard
            .discovery
            .streaming_checked_entry_paths_in_directory(relative_directory)?;
        Ok(PublicationGuardedMetadataInventoryEntries {
            entries,
            _guard: guard,
        })
    }

    pub(crate) fn inspect_media(
        &self,
        inspector: &super::media_inspector::LocalMediaInspector,
        file: &DiscoveredFile,
    ) -> Result<MediaInspection, crate::ports::MediaInspectionFailure> {
        inspector.inspect_with_discovery(&self.discovery, file)
    }

    pub(crate) fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue> {
        self.discovery
            .revalidate_relative_file_state(relative_path, expected)
    }

    fn try_clone_capability(&self) -> std::io::Result<Self> {
        Ok(Self {
            discovery: FileDiscovery {
                root: self.discovery.root.clone(),
                canonical_root: self.discovery.canonical_root.clone(),
                #[cfg(windows)]
                root_proof: self.discovery.root_proof.try_clone_capability()?,
                #[cfg(windows)]
                _publication_namespace_ancestors: self
                    .discovery
                    ._publication_namespace_ancestors
                    .iter()
                    .map(File::try_clone)
                    .collect::<Result<Vec<_>, _>>()?,
            },
        })
    }
}

impl FileDiscovery {
    pub fn canonical_root(&self) -> Result<PathBuf, ScanError> {
        Ok(self.canonical_root.clone())
    }

    pub(crate) fn metadata_inventory_root_identity(
        &self,
    ) -> Result<Option<FileIdentityEvidence>, ScanError> {
        #[cfg(windows)]
        {
            let identity =
                file_identity_from_handle(self.root_proof.identity_handle()).map_err(|error| {
                    ScanError::new(
                        "root_identity_unavailable",
                        format!("Could not revalidate the pinned source root identity: {error}"),
                    )
                })?;
            if identity.as_ref() != Some(&self.root_proof.identity) {
                return Err(ScanError::new(
                    "root_identity_changed",
                    "The pinned source root identity changed during recovery",
                ));
            }
            Ok(identity)
        }
        #[cfg(not(windows))]
        {
            Ok(None)
        }
    }

    pub(crate) fn require_metadata_inventory_root_identity(
        &self,
        expected: &FileIdentityEvidence,
    ) -> Result<(), ScanError> {
        if self.metadata_inventory_root_identity()?.as_ref() == Some(expected) {
            Ok(())
        } else {
            Err(ScanError::new(
                "metadata_inventory_root_identity_changed",
                "The recovery source root no longer matches its durable handle identity",
            ))
        }
    }

    pub fn entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<DirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
            return Err(path_containment_issue(&directory_path));
        }
        let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_unreadable".to_owned(),
            message: error.to_string(),
        })?;
        Ok(DirectoryEntryPaths {
            root: self.root.clone(),
            directory_path,
            entries,
            index: 0,
        })
    }

    pub(crate) fn checked_entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<CheckedDirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
            return Err(path_containment_issue(&directory_path));
        }
        let directory_identity = file_identity(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_unreadable".to_owned(),
            message: error.to_string(),
        })?;
        let entries = stable_checked_directory_entries(&self.root, &directory_path, entries)?;
        #[cfg(test)]
        {
            let root_path = path_text(&self.root);
            if let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(&root_path)
            {
                counts.directory_opens = counts.directory_opens.saturating_add(1);
                counts.entry_reads = counts
                    .entry_reads
                    .saturating_add(u64::try_from(entries.len()).unwrap_or(u64::MAX));
            }
        }
        let closing_identity = file_identity(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if directory_identity != closing_identity {
            return Err(ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its stable frontier was opened"
                    .to_owned(),
            });
        }
        Ok(CheckedDirectoryEntryPaths {
            entries,
            next_index: 0,
            #[cfg(test)]
            directory_identity,
        })
    }

    fn streaming_checked_entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<StreamingCheckedDirectoryEntryPaths, ScanIssue> {
        self.streaming_checked_entry_paths_in_directory_with_hook(relative_directory, || {})
    }

    fn streaming_checked_entry_paths_in_directory_with_hook(
        &self,
        relative_directory: &str,
        after_root_proof: impl FnOnce(),
    ) -> Result<StreamingCheckedDirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        #[cfg(not(windows))]
        {
            let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
            if !metadata.is_dir()
                || (!relative_directory_path.as_os_str().is_empty()
                    && is_link_or_reparse_point(&metadata))
            {
                return Err(path_containment_issue(&directory_path));
            }
            let directory_identity = file_identity(&directory_path).map_err(|error| ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
            let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            after_root_proof();
            #[cfg(test)]
            let test_counter_root = {
                let root_path = path_text(&self.root);
                let mut instrumentation = SOURCE_ENUMERATION_COUNTS
                    .lock()
                    .expect("source enumeration instrumentation");
                if let Some(counts) = instrumentation.get_mut(&root_path) {
                    counts.directory_opens = counts.directory_opens.saturating_add(1);
                    Some(root_path)
                } else {
                    None
                }
            };
            Ok(StreamingCheckedDirectoryEntryPaths {
                root: self.root.clone(),
                directory_path,
                entries,
                directory_identity,
                #[cfg(test)]
                test_counter_root,
            })
        }
        #[cfg(windows)]
        {
            let (directory, directory_identity) =
                self.open_handle_anchored_directory(relative_directory_path, after_root_proof)?;
            #[cfg(test)]
            let test_counter_root = {
                let root_path = path_text(&self.root);
                let mut instrumentation = SOURCE_ENUMERATION_COUNTS
                    .lock()
                    .expect("source enumeration instrumentation");
                if let Some(counts) = instrumentation.get_mut(&root_path) {
                    counts.directory_opens = counts.directory_opens.saturating_add(1);
                    Some(root_path)
                } else {
                    None
                }
            };
            Ok(StreamingCheckedDirectoryEntryPaths {
                directory,
                directory_relative_path: relative_path_text(relative_directory_path),
                buffer: vec![0_u64; HANDLE_DIRECTORY_BUFFER_BYTES / size_of::<u64>()]
                    .into_boxed_slice(),
                pending_entries: VecDeque::new(),
                observed_paths: HashSet::new(),
                volume_serial: self.root_proof.volume_serial,
                should_restart: true,
                enumeration_complete: false,
                enumeration_failed: false,
                directory_identity: Some(directory_identity),
                #[cfg(test)]
                test_counter_root,
            })
        }
    }

    #[cfg(windows)]
    fn open_handle_anchored_directory(
        &self,
        relative_directory: &Path,
        after_root_proof: impl FnOnce(),
    ) -> Result<(File, FileIdentityEvidence), ScanIssue> {
        let root_identity =
            file_identity_from_handle(self.root_proof.identity_handle()).map_err(|error| {
                ScanIssue {
                    path: Some(path_text(&self.canonical_root)),
                    code: "root_identity_unavailable".to_owned(),
                    message: error.to_string(),
                }
            })?;
        if root_identity.as_ref() != Some(&self.root_proof.identity) {
            return Err(ScanIssue {
                path: Some(path_text(&self.canonical_root)),
                code: "root_identity_changed".to_owned(),
                message: "The authorized source root identity changed during inventory".to_owned(),
            });
        }
        let expected_path = self.canonical_root.join(relative_directory);
        after_root_proof();
        let directory = open_root_relative_handle(
            &self.root_proof.handle,
            relative_directory,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            Some(true),
        )
        .map_err(|error| {
            if error.raw_os_error()
                == Some(i32::try_from(ERROR_REPARSE_POINT_ENCOUNTERED).expect("Win32 error fits"))
            {
                path_containment_issue(&expected_path)
            } else {
                ScanIssue {
                    path: Some(path_text(&expected_path)),
                    code: "directory_unreadable".to_owned(),
                    message: error.to_string(),
                }
            }
        })?;
        let info = file_attribute_tag_info_from_handle(&directory).map_err(|error| ScanIssue {
            path: Some(path_text(&expected_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            || info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(path_containment_issue(&expected_path));
        }
        let identity = file_identity_from_handle(&directory)
            .map_err(|error| ScanIssue {
                path: Some(path_text(&expected_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?
            .ok_or_else(|| ScanIssue {
                path: Some(path_text(&expected_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: "The directory handle has no stable Windows file identity".to_owned(),
            })?;
        if relative_directory.as_os_str().is_empty() && identity != self.root_proof.identity {
            return Err(path_containment_issue(&expected_path));
        }
        Ok((directory, identity))
    }

    #[cfg(test)]
    pub(crate) fn metadata_inventory_directory_identity(
        &self,
        relative_directory: &str,
    ) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !metadata.is_dir()
            || (!relative_directory_path.as_os_str().is_empty()
                && is_link_or_reparse_point(&metadata))
        {
            return Err(path_containment_issue(&directory_path));
        }
        file_identity(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })
    }

    pub fn visit_relative_path(&self, relative_path: &str) -> FileVisit {
        let relative_path = match validated_relative_path(relative_path) {
            Ok(path) => path.to_path_buf(),
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path.to_owned(),
                    outcome: FileVisitOutcome::Issue(error),
                };
            }
        };
        #[cfg(windows)]
        {
            self.visit_root_relative_path(relative_path)
        }
        #[cfg(not(windows))]
        {
            let (path, metadata) = match self.checked_existing_path(&relative_path) {
                Ok(resolved) => resolved,
                Err(issue) => {
                    return FileVisit {
                        relative_path: relative_path_text(relative_path),
                        outcome: FileVisitOutcome::Issue(issue),
                    };
                }
            };
            let relative_path = relative_path_text(relative_path);
            match checked_directory_entry_from_metadata(relative_path.clone(), &path, metadata) {
                Ok(entry) => self.visit_directory_entry(entry),
                Err(issue) => FileVisit {
                    relative_path,
                    outcome: FileVisitOutcome::Issue(issue),
                },
            }
        }
    }

    #[cfg(windows)]
    fn visit_root_relative_path(&self, relative_path: PathBuf) -> FileVisit {
        let relative_path_text = relative_path_text(&relative_path);
        let path = self.canonical_root.join(&relative_path);
        let handle = match open_root_relative_handle(
            &self.root_proof.handle,
            &relative_path,
            FILE_READ_ATTRIBUTES,
            None,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path_text,
                    outcome: FileVisitOutcome::Issue(root_relative_path_issue(&path, error)),
                };
            }
        };
        let metadata = match handle.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path_text,
                    outcome: FileVisitOutcome::Issue(path_metadata_issue(&path, error)),
                };
            }
        };
        let attributes = match file_attribute_tag_info_from_handle(&handle) {
            Ok(attributes) => attributes,
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path_text,
                    outcome: FileVisitOutcome::Issue(ScanIssue {
                        path: Some(path_text(&path)),
                        code: "file_reparse_evidence_unreadable".to_owned(),
                        message: error.to_string(),
                    }),
                };
            }
        };
        let (reparse_kind, placeholder_state) = match reparse_evidence_from_attribute_tag(
            attributes.FileAttributes,
            attributes.ReparseTag,
        ) {
            Ok(evidence) => evidence,
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path_text,
                    outcome: FileVisitOutcome::Issue(ScanIssue {
                        path: Some(path_text(&path)),
                        code: "file_reparse_evidence_unreadable".to_owned(),
                        message: error.to_string(),
                    }),
                };
            }
        };
        let identity = file_identity_from_handle(&handle).ok().flatten();
        self.visit_relative_path_with_metadata(
            relative_path_text,
            path,
            metadata,
            reparse_kind,
            placeholder_state,
            identity,
        )
    }

    pub(crate) fn visit_directory_entry(&self, entry: CheckedDirectoryEntry) -> FileVisit {
        let relative_path = entry.relative_path;
        let path = self.root.join(Path::new(&relative_path));
        self.visit_relative_path_with_metadata(
            relative_path,
            path,
            entry.metadata,
            entry.reparse_kind,
            entry.placeholder_state,
            None,
        )
    }

    #[cfg(windows)]
    pub(crate) fn open_pinned_source_file(&self, relative_path: &str) -> std::io::Result<File> {
        self.open_pinned_source_file_with_hook(relative_path, || {})
    }

    #[cfg(windows)]
    fn open_pinned_source_file_with_hook(
        &self,
        relative_path: &str,
        after_handle_open: impl FnOnce(),
    ) -> std::io::Result<File> {
        let relative_path = validated_relative_path(relative_path)
            .map_err(|issue| std::io::Error::other(issue.message))?;
        let metadata_handle = open_root_relative_handle(
            &self.root_proof.handle,
            relative_path,
            FILE_READ_ATTRIBUTES,
            Some(false),
        )?;
        after_handle_open();
        let info = file_attribute_tag_info_from_handle(&metadata_handle)?;
        let evidence = AttributeTagEvidence {
            attributes: info.FileAttributes,
            reparse_tag: info.ReparseTag,
        };
        open_validated_root_relative_source_file(
            &self.root,
            &self.root_proof,
            relative_path,
            &metadata_handle,
            evidence,
        )
    }

    pub(crate) fn metadata_inventory_entry(
        &self,
        relative_path: &str,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        let relative_path_value = validated_relative_path(relative_path)?;
        let (path, metadata) = self.checked_existing_path(relative_path_value)?;
        checked_directory_entry_from_metadata(
            relative_path_text(relative_path_value),
            &path,
            metadata,
        )
        .and_then(|entry| self.metadata_inventory_entry_from_directory_entry(entry))
    }

    pub(crate) fn metadata_inventory_entry_from_directory_entry(
        &self,
        entry: CheckedDirectoryEntry,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        let relative_path_value = validated_relative_path(&entry.relative_path)?;
        let path = self.root.join(relative_path_value);
        metadata_inventory_entry_from_checked_directory_entry(entry, &path)
    }

    pub fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue> {
        let relative_path = validated_relative_path(relative_path)?;
        #[cfg(windows)]
        {
            let path = self.canonical_root.join(relative_path);
            let file = open_root_relative_handle_with_options(
                &self.root_proof.handle,
                relative_path,
                FILE_READ_ATTRIBUTES,
                Some(false),
                true,
                true,
            )
            .map_err(|error| path_metadata_issue(&path, error))?;
            let info = file_attribute_tag_info_from_handle(&file).map_err(|error| ScanIssue {
                path: Some(path_text(&path)),
                code: "source_revalidation_failed".to_owned(),
                message: error.to_string(),
            })?;
            validate_present_file_revalidation_attributes(&path, &info)?;
            let metadata = file
                .metadata()
                .map_err(|error| path_metadata_issue(&path, error))?;
            let actual_identity = file_identity_from_handle(&file).map_err(|error| ScanIssue {
                path: Some(path_text(&path)),
                code: "source_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
            let actual_revision =
                source_revision_from_handle(&file).map_err(|error| ScanIssue {
                    path: Some(path_text(&path)),
                    code: "source_revision_unavailable".to_owned(),
                    message: error.to_string(),
                })?;
            revalidate_file_state_values(
                expected,
                &metadata,
                actual_identity,
                Some(actual_revision),
            )
        }
        #[cfg(not(windows))]
        {
            let (path, metadata) = self.checked_existing_path(relative_path)?;
            let evidence = checked_directory_entry_from_metadata(
                relative_path_text(relative_path),
                &path,
                metadata,
            )?;
            if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
                return Err(path_containment_issue(&path));
            }
            revalidate_file_state_with_metadata(expected, &path, &evidence)
        }
    }

    fn checked_existing_path(
        &self,
        relative_path: &Path,
    ) -> Result<(PathBuf, Metadata), ScanIssue> {
        let mut current = self.root.clone();
        let mut components = relative_path.components().peekable();
        while let Some(component) = components.next() {
            current.push(component.as_os_str());
            if components.peek().is_none() {
                break;
            }
            match current.symlink_metadata() {
                Ok(metadata) if is_link_or_reparse_point(&metadata) => {
                    return Err(path_containment_issue(&current));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(path_metadata_issue(&current, error)),
            }
        }
        let metadata = current
            .symlink_metadata()
            .map_err(|error| path_metadata_issue(&current, error))?;
        if !is_link_or_reparse_point(&metadata) {
            let canonical = current
                .canonicalize()
                .map_err(|error| path_metadata_issue(&current, error))?;
            if !canonical.starts_with(&self.canonical_root) {
                return Err(path_containment_issue(&current));
            }
        }
        Ok((current, metadata))
    }
}

#[cfg(windows)]
fn windows_path_uses_device_namespace(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::DeviceNS(_) | Prefix::Verbatim(_))
    )
}

fn checked_directory_entry_from_metadata(
    relative_path: String,
    path: &Path,
    metadata: Metadata,
) -> Result<CheckedDirectoryEntry, ScanIssue> {
    let (reparse_kind, placeholder_state) =
        entry_reparse_evidence(path, &metadata).map_err(|error| ScanIssue {
            path: Some(path_text(path)),
            code: "file_reparse_evidence_unreadable".to_owned(),
            message: error.to_string(),
        })?;
    Ok(CheckedDirectoryEntry {
        relative_path,
        metadata,
        reparse_kind,
        placeholder_state,
    })
}

fn metadata_inventory_entry_from_checked_directory_entry(
    entry: CheckedDirectoryEntry,
    path: &Path,
) -> Result<MetadataInventoryEntry, ScanIssue> {
    let is_reparse_point = entry.reparse_kind != ReparseKind::None;
    let is_opaque_directory = entry.metadata.is_dir()
        && (is_reparse_point
            || entry.placeholder_state != MetadataInventoryPlaceholderState::Available);
    let kind = if entry.reparse_kind == ReparseKind::Other || is_opaque_directory {
        MetadataInventoryEntryKind::Other
    } else {
        metadata_inventory_entry_kind(
            entry.metadata.is_dir(),
            entry.metadata.is_file(),
            entry.reparse_kind,
        )
    };
    let (file_identity, source_revision) = if kind == MetadataInventoryEntryKind::File
        && entry.placeholder_state == MetadataInventoryPlaceholderState::Available
    {
        #[cfg(windows)]
        {
            file_source_evidence(path)
                .map(|(identity, revision)| (identity, Some(revision)))
                .unwrap_or((None, None))
        }
        #[cfg(not(windows))]
        {
            (file_identity(path).ok().flatten(), None)
        }
    } else {
        (None, None)
    };
    Ok(MetadataInventoryEntry {
        relative_path: entry.relative_path,
        kind,
        file_size: (kind == MetadataInventoryEntryKind::File).then_some(entry.metadata.len()),
        modified_unix_ms: modified_unix_ms(&entry.metadata),
        file_identity,
        source_revision,
        placeholder_state: entry.placeholder_state,
        is_reparse_point,
    })
}

#[cfg(windows)]
fn entry_reparse_evidence(
    path: &Path,
    metadata: &Metadata,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    let metadata_state = metadata_placeholder_state(metadata);
    if !has_reparse_point_attribute(metadata.file_attributes()) {
        return Ok((ReparseKind::None, metadata_state));
    }
    let validation = open_validation_handle(path)?;
    let evidence = exact_attribute_tag_evidence(path, &validation, metadata.file_attributes())?;
    reparse_evidence_from_attribute_tag(evidence.attributes, evidence.reparse_tag)
}

#[cfg(windows)]
fn reparse_evidence_from_attribute_tag(
    attributes: u32,
    reparse_tag: u32,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    let placeholder_state = metadata_placeholder_state_from_attributes(attributes);
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return Ok((ReparseKind::None, placeholder_state));
    }
    let cloud_state = cloud_placeholder_state_from_attribute_tag(attributes, reparse_tag)?;
    if cloud_state & CF_PLACEHOLDER_STATE_PLACEHOLDER == 0 {
        return Ok((ReparseKind::Other, placeholder_state));
    }
    let placeholder_state = if cloud_state
        & (CF_PLACEHOLDER_STATE_PARTIAL | CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK)
        != 0
        && placeholder_state == MetadataInventoryPlaceholderState::Available
    {
        MetadataInventoryPlaceholderState::RecallOnDataAccess
    } else {
        placeholder_state
    };
    Ok((ReparseKind::CloudFiles, placeholder_state))
}

#[cfg(windows)]
fn validate_present_file_revalidation_attributes(
    path: &Path,
    info: &FILE_ATTRIBUTE_TAG_INFO,
) -> Result<(), ScanIssue> {
    if info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Err(path_containment_issue(path));
    }
    let (reparse_kind, placeholder_state) =
        reparse_evidence_from_attribute_tag(info.FileAttributes, info.ReparseTag).map_err(
            |error| ScanIssue {
                path: Some(path_text(path)),
                code: "file_reparse_evidence_unreadable".to_owned(),
                message: error.to_string(),
            },
        )?;
    if placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(ScanIssue {
            path: Some(path_text(path)),
            code: "cloud_placeholder_skipped".to_owned(),
            message: "The file is not locally available and was not hydrated".to_owned(),
        });
    }
    if reparse_kind == ReparseKind::Other {
        return Err(path_containment_issue(path));
    }
    Ok(())
}

#[cfg(not(windows))]
fn entry_reparse_evidence(
    _path: &Path,
    metadata: &Metadata,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    Ok((
        if metadata.file_type().is_symlink() {
            ReparseKind::Other
        } else {
            ReparseKind::None
        },
        MetadataInventoryPlaceholderState::Available,
    ))
}

fn metadata_inventory_entry_kind(
    is_directory: bool,
    is_regular_file: bool,
    reparse_kind: ReparseKind,
) -> MetadataInventoryEntryKind {
    if is_directory {
        MetadataInventoryEntryKind::Directory
    } else if is_regular_file && reparse_kind != ReparseKind::Other {
        MetadataInventoryEntryKind::File
    } else {
        MetadataInventoryEntryKind::Other
    }
}

fn path_metadata_issue(path: &Path, error: std::io::Error) -> ScanIssue {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => "file_missing",
        std::io::ErrorKind::PermissionDenied => "file_inaccessible",
        _ => "file_metadata_unreadable",
    };
    ScanIssue {
        path: Some(path_text(path)),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(windows)]
fn root_relative_path_issue(path: &Path, error: std::io::Error) -> ScanIssue {
    if error
        .get_ref()
        .is_some_and(|source| source.is::<RootRelativeContainmentViolation>())
    {
        path_containment_issue(path)
    } else {
        path_metadata_issue(path, error)
    }
}

fn path_containment_issue(path: &Path) -> ScanIssue {
    ScanIssue {
        path: Some(path_text(path)),
        code: "source_path_outside_root".to_owned(),
        message: "The source path crossed a filesystem link outside the selected root".to_owned(),
    }
}

fn is_link_or_reparse_point(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        has_reparse_point_attribute(metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn has_reparse_point_attribute(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn validated_relative_path(value: &str) -> Result<&Path, ScanIssue> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ScanIssue {
            path: Some(value.to_owned()),
            code: "directory_frontier_invalid".to_owned(),
            message: "The stored directory entry escaped the selected root".to_owned(),
        });
    }
    Ok(path)
}

pub fn revalidate_file_state(expected: &ExpectedFileState) -> Result<(), ScanIssue> {
    let path = Path::new(&expected.absolute_path);
    let metadata = path.symlink_metadata().map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "source_revalidation_failed".to_owned(),
        message: error.to_string(),
    })?;
    let evidence = checked_directory_entry_from_metadata(String::new(), path, metadata)?;
    if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
        return Err(path_containment_issue(path));
    }
    revalidate_file_state_with_metadata(expected, path, &evidence)
}

pub(crate) struct OpenedPreviewSource {
    pub(crate) file: File,
    pub(crate) source_revision: Option<SourceRevisionEvidence>,
    pub(crate) source_root_path: PathBuf,
}

pub(crate) struct PreviewPublicationGuard {
    source_file: File,
    #[cfg(windows)]
    _root_proof: WindowsDirectoryRootProof,
    #[cfg(windows)]
    _namespace_guards: Vec<File>,
}

impl PreviewPublicationGuard {
    pub(crate) fn source_file(&self) -> &File {
        &self.source_file
    }
}

pub(crate) fn open_preview_source(
    expected: &ExpectedFileState,
    source_root: &Path,
    expected_root_identity: Option<&FileIdentityEvidence>,
) -> Result<OpenedPreviewSource, ScanIssue> {
    let path = Path::new(&expected.absolute_path);

    #[cfg(windows)]
    let (file, source_root_path) = {
        let expected_root_identity = expected_root_identity.ok_or_else(|| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "preview_root_identity_unproven".to_owned(),
            message: "The preview source root lacks durable Windows identity evidence".to_owned(),
        })?;
        let (root_proof, canonical_root) =
            WindowsDirectoryRootProof::open_configured(source_root, true).map_err(|error| {
                ScanIssue {
                    path: Some(expected.absolute_path.clone()),
                    code: "preview_root_unavailable".to_owned(),
                    message: format!("The preview source root cannot be resolved safely: {error}"),
                }
            })?;
        if &root_proof.identity != expected_root_identity {
            return Err(ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_root_identity_changed".to_owned(),
                message: "The preview source root no longer matches its publication identity"
                    .to_owned(),
            });
        }
        let file = open_source_file_with_root_proof(path, &canonical_root, &root_proof, || {})
            .map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_source_open_failed".to_owned(),
                message: error.to_string(),
            })?;
        (file, canonical_root)
    };

    #[cfg(not(windows))]
    let (file, source_root_path) = {
        let source_root_path =
            canonical_source_root_path(source_root).map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_root_unavailable".to_owned(),
                message: format!("The preview source root cannot be resolved safely: {error}"),
            })?;
        let file = open_source_file(path, &source_root_path).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "preview_source_open_failed".to_owned(),
            message: error.to_string(),
        })?;
        (file, source_root_path)
    };

    #[cfg(not(windows))]
    let _ = expected_root_identity;
    let source_revision = revalidate_open_preview_source(&file, expected)?;
    Ok(OpenedPreviewSource {
        file,
        source_revision,
        source_root_path,
    })
}

pub(crate) fn revalidate_open_preview_source(
    file: &File,
    expected: &ExpectedFileState,
) -> Result<Option<SourceRevisionEvidence>, ScanIssue> {
    let metadata = file.metadata().map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "source_revalidation_failed".to_owned(),
        message: error.to_string(),
    })?;
    #[cfg(windows)]
    let (actual_identity, actual_revision) = (
        file_identity_from_handle(file).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?,
        Some(
            source_revision_from_handle(file).map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "source_revision_unavailable".to_owned(),
                message: error.to_string(),
            })?,
        ),
    );
    #[cfg(not(windows))]
    let (actual_identity, actual_revision) = (None, None);
    revalidate_file_state_values(
        expected,
        &metadata,
        actual_identity,
        actual_revision.clone(),
    )?;
    Ok(actual_revision)
}

#[cfg(windows)]
pub(crate) fn open_preview_publication_guard(
    expected: &ExpectedFileState,
    source_root: &Path,
    expected_root_identity: Option<&FileIdentityEvidence>,
) -> Result<PreviewPublicationGuard, ScanIssue> {
    let path = Path::new(&expected.absolute_path);
    let (root_proof, _, namespace_guards) =
        WindowsDirectoryRootProof::open_publication_namespace(source_root, expected_root_identity)
            .map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_source_guard_failed".to_owned(),
                message: format!("Could not pin the preview source root namespace: {error}"),
            })?;
    let file = OpenOptions::new()
        .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "preview_source_guard_failed".to_owned(),
            message: format!("Could not pin the preview source file: {error}"),
        })?;
    let info = file_attribute_tag_info_from_handle(&file).map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "preview_source_guard_failed".to_owned(),
        message: format!("Could not validate the pinned preview source attributes: {error}"),
    })?;
    validate_source_file_info(&AttributeTagEvidence {
        attributes: info.FileAttributes,
        reparse_tag: info.ReparseTag,
    })
    .map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "preview_source_guard_failed".to_owned(),
        message: error.to_string(),
    })?;
    validate_handle_within_root(&file, &root_proof).map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "preview_source_guard_failed".to_owned(),
        message: format!("Could not validate the pinned preview source root: {error}"),
    })?;
    revalidate_open_preview_source(&file, expected)?;
    Ok(PreviewPublicationGuard {
        source_file: file,
        _root_proof: root_proof,
        _namespace_guards: namespace_guards,
    })
}

#[cfg(not(windows))]
pub(crate) fn open_preview_publication_guard(
    expected: &ExpectedFileState,
    source_root: &Path,
    _expected_root_identity: Option<&FileIdentityEvidence>,
) -> Result<PreviewPublicationGuard, ScanIssue> {
    let opened = open_preview_source(expected, source_root, None)?;
    Ok(PreviewPublicationGuard {
        source_file: opened.file,
    })
}

fn revalidate_file_state_with_metadata(
    expected: &ExpectedFileState,
    path: &Path,
    evidence: &CheckedDirectoryEntry,
) -> Result<(), ScanIssue> {
    if evidence.placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_became_unavailable".to_owned(),
            message: "The file is no longer locally available".to_owned(),
        });
    }
    let actual_identity = if expected.file_identity.is_some() {
        file_identity(path).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?
    } else {
        None
    };
    #[cfg(windows)]
    let actual_revision = file_source_evidence(path)
        .map(|(_, revision)| Some(revision))
        .map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_revision_unavailable".to_owned(),
            message: error.to_string(),
        })?;
    #[cfg(not(windows))]
    let actual_revision = None;
    revalidate_file_state_values(
        expected,
        &evidence.metadata,
        actual_identity,
        actual_revision,
    )
}

fn revalidate_file_state_values(
    expected: &ExpectedFileState,
    metadata: &Metadata,
    actual_identity: Option<FileIdentityEvidence>,
    actual_revision: Option<SourceRevisionEvidence>,
) -> Result<(), ScanIssue> {
    if metadata.len() != expected.file_size
        || modified_unix_ms(metadata) != expected.modified_unix_ms
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_changed_during_scan".to_owned(),
            message: "The file size or modification time changed during the scan".to_owned(),
        });
    }
    if let Some(expected_identity) = &expected.file_identity
        && actual_identity.as_ref() != Some(expected_identity)
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_replaced_during_scan".to_owned(),
            message: "The file identity changed during the scan".to_owned(),
        });
    }
    if let Some(expected_revision) = &expected.source_revision
        && actual_revision.as_ref() != Some(expected_revision)
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_revision_changed_during_scan".to_owned(),
            message: "The filesystem source revision changed during the scan".to_owned(),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn file_identity(path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    let file = open_validation_handle(path)?;
    file_identity_from_handle(&file)
}

#[cfg(test)]
pub(crate) fn file_identity_evidence(path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    file_identity(path)
}

#[cfg(windows)]
fn file_identity_from_handle(file: &File) -> std::io::Result<Option<FileIdentityEvidence>> {
    let info = raw_file_id_info(file)?;
    let file_id = u128::from_le_bytes(info.FileId.Identifier);
    Ok(Some(FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("{:016x}:{file_id:032x}", info.VolumeSerialNumber),
    }))
}

#[cfg(windows)]
fn windows_source_revision(change_time_100ns: i64) -> SourceRevisionEvidence {
    SourceRevisionEvidence {
        scheme: WINDOWS_SOURCE_REVISION_SCHEME.to_owned(),
        value: format!(
            "{:016x}",
            u64::from_ne_bytes(change_time_100ns.to_ne_bytes())
        ),
    }
}

#[cfg(windows)]
fn source_revision_from_handle(file: &File) -> std::io::Result<SourceRevisionEvidence> {
    let mut info = FILE_BASIC_INFO::default();
    // SAFETY: the borrowed File keeps the handle live and `info` is a correctly sized writable
    // FILE_BASIC_INFO buffer for the duration of this synchronous attribute-only call.
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_BASIC_INFO>()).expect("FILE_BASIC_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(windows_source_revision(info.ChangeTime))
}

#[cfg(windows)]
fn file_source_evidence(
    path: &Path,
) -> std::io::Result<(Option<FileIdentityEvidence>, SourceRevisionEvidence)> {
    let file = open_validation_handle(path)?;
    Ok((
        file_identity_from_handle(&file)?,
        source_revision_from_handle(&file)?,
    ))
}

#[cfg(windows)]
fn raw_file_id_info(file: &File) -> std::io::Result<FILE_ID_INFO> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: ADR 0007 fixes the complete contract: the live file owns the handle for this call,
    // `info` is aligned and writable, and the buffer length exactly matches `FILE_ID_INFO`.
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_ID_INFO>()).expect("FILE_ID_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info)
}

#[cfg(windows)]
fn open_validation_handle(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT,
        )
        .open(path)
}

#[cfg(windows)]
fn open_root_relative_handle(
    root: &File,
    relative_path: &Path,
    terminal_access: u32,
    terminal_is_directory: Option<bool>,
) -> std::io::Result<File> {
    open_root_relative_handle_with_options(
        root,
        relative_path,
        terminal_access,
        terminal_is_directory,
        true,
        false,
    )
}

#[cfg(windows)]
fn open_root_relative_data_handle(root: &File, relative_path: &Path) -> std::io::Result<File> {
    open_root_relative_handle_with_options(
        root,
        relative_path,
        FILE_READ_DATA | FILE_READ_ATTRIBUTES,
        Some(false),
        false,
        true,
    )
}

#[cfg(windows)]
fn open_root_relative_handle_with_options(
    root: &File,
    relative_path: &Path,
    terminal_access: u32,
    terminal_is_directory: Option<bool>,
    share_terminal_delete: bool,
    prevent_terminal_recall: bool,
) -> std::io::Result<File> {
    let components = relative_path
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            Component::CurDir => Err(std::io::Error::other(
                "Root-relative source paths cannot contain a current-directory component",
            )),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => Err(
                std::io::Error::other("The root-relative source path escaped its pinned root"),
            ),
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    if components.is_empty() {
        return root.try_clone();
    }

    let component_count = components.len();
    let mut parent = root.try_clone()?;
    for (index, component) in components.into_iter().enumerate() {
        let is_terminal = index + 1 == component_count;
        let access = if is_terminal {
            terminal_access
        } else {
            FILE_TRAVERSE | FILE_READ_ATTRIBUTES
        } | SYNCHRONIZE;
        let is_directory = (!is_terminal).then_some(true).or(terminal_is_directory);
        let opened = nt_create_root_relative_handle(
            &parent,
            component,
            access,
            is_directory,
            !is_terminal || share_terminal_delete,
            is_terminal && prevent_terminal_recall,
        )
        .map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!(
                    "Could not open root-relative component {}: {error}",
                    component.to_string_lossy()
                ),
            )
        })?;
        let attributes = file_attribute_tag_info_from_handle(&opened)?;
        if !is_terminal && attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || is_directory == Some(true)
                && attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            || is_directory == Some(false)
                && attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0
        {
            return Err(std::io::Error::other(RootRelativeContainmentViolation));
        }
        parent = opened;
    }
    Ok(parent)
}

#[cfg(windows)]
fn nt_create_root_relative_handle(
    parent: &File,
    component: &std::ffi::OsStr,
    desired_access: u32,
    is_directory: Option<bool>,
    share_delete: bool,
    prevent_recall: bool,
) -> std::io::Result<File> {
    let is_case_sensitive = directory_is_case_sensitive(parent).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Could not query parent case semantics: {error}"),
        )
    })?;
    nt_create_root_relative_handle_with_case_semantics(
        parent,
        component,
        desired_access,
        is_directory,
        share_delete,
        prevent_recall,
        is_case_sensitive,
    )
}

#[cfg(windows)]
unsafe fn nt_create_root_relative_raw(
    handle: *mut HANDLE,
    desired_access: u32,
    object_attributes: *const OBJECT_ATTRIBUTES,
    io_status: *mut IO_STATUS_BLOCK,
    share_access: u32,
    create_options: u32,
) -> i32 {
    // SAFETY: the caller owns the live buffers and handles described by ADR 0024. This wrapper
    // fixes the EA pair to null/zero so Win32 flags cannot be confused with the native EA length.
    unsafe {
        NtCreateFileRaw(
            handle,
            desired_access,
            object_attributes,
            io_status,
            std::ptr::null(),
            0,
            share_access,
            FILE_OPEN,
            create_options,
            std::ptr::null(),
            0,
        )
    }
}

#[cfg(windows)]
fn nt_create_root_relative_handle_with_case_semantics(
    parent: &File,
    component: &std::ffi::OsStr,
    desired_access: u32,
    is_directory: Option<bool>,
    share_delete: bool,
    prevent_recall: bool,
    is_case_sensitive: bool,
) -> std::io::Result<File> {
    let mut name = component.encode_wide().collect::<Vec<_>>();
    if name.is_empty()
        || name
            .iter()
            .any(|unit| *unit == 0 || *unit == u16::from(b'\\'))
    {
        return Err(std::io::Error::other(
            "The root-relative source component is not a single valid name",
        ));
    }
    let byte_len = name
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or_else(|| std::io::Error::other("The root-relative source name is too long"))?;
    let unicode_name = UNICODE_STRING {
        Length: byte_len,
        MaximumLength: byte_len,
        Buffer: name.as_mut_ptr(),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>())
            .expect("OBJECT_ATTRIBUTES size fits u32"),
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &raw const unicode_name,
        Attributes: OBJ_DONT_REPARSE
            | if is_case_sensitive {
                0
            } else {
                OBJ_CASE_INSENSITIVE
            },
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut handle = INVALID_HANDLE_VALUE;
    let mut io_status = IO_STATUS_BLOCK::default();
    let type_option = match is_directory {
        Some(true) => FILE_DIRECTORY_FILE,
        Some(false) => FILE_NON_DIRECTORY_FILE,
        None => 0,
    };
    let share_access =
        FILE_SHARE_READ | FILE_SHARE_WRITE | if share_delete { FILE_SHARE_DELETE } else { 0 };
    let create_options = type_option
        | if desired_access == 0 {
            0
        } else {
            FILE_OPEN_FOR_BACKUP_INTENT
        }
        | FILE_OPEN_REPARSE_POINT
        | if prevent_recall {
            FILE_OPEN_NO_RECALL_NATIVE
        } else {
            0
        }
        | if desired_access & SYNCHRONIZE != 0 {
            FILE_SYNCHRONOUS_IO_NONALERT
        } else {
            0
        };
    #[cfg(test)]
    {
        let facts = RootRelativeNtOpenFacts {
            desired_access,
            share_access,
            create_options,
            ea_buffer_is_null: true,
            ea_length: 0,
            object_name_is_absolute: Path::new(component).is_absolute(),
            object_name_has_separator: component
                .encode_wide()
                .any(|unit| unit == u16::from(b'\\') || unit == u16::from(b'/')),
        };
        if create_options & FILE_OPEN_NO_RECALL_NATIVE != 0 {
            LAST_ROOT_RELATIVE_DATA_OPEN.with(|slot| slot.set(Some(facts)));
        } else if create_options & FILE_NON_DIRECTORY_FILE != 0 {
            LAST_ROOT_RELATIVE_METADATA_OPEN.with(|slot| slot.set(Some(facts)));
        }
    }
    // SAFETY: ADR 0024 owns this Windows 11 x64 adapter boundary. `parent` owns and keeps the
    // RootDirectory handle live; the UTF-16 buffer, UNICODE_STRING, OBJECT_ATTRIBUTES, output
    // handle, and IO_STATUS_BLOCK remain live with their generated layouts for the synchronous
    // call. No pointer escapes. A successful raw handle is transferred exactly once into `File`.
    let status = unsafe {
        nt_create_root_relative_raw(
            &raw mut handle,
            desired_access,
            &raw const object_attributes,
            &raw mut io_status,
            share_access,
            create_options,
        )
    };
    if status < 0 {
        // SAFETY: converting an NTSTATUS value does not dereference pointers or retain state.
        let error = unsafe { RtlNtStatusToDosError(status) };
        return Err(std::io::Error::from_raw_os_error(
            i32::try_from(error).unwrap_or(i32::MAX),
        ));
    }
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::other(
            "NtCreateFile succeeded without returning a valid source handle",
        ));
    }
    // SAFETY: `handle` is a successful, uniquely returned NtCreateFile handle and has no other
    // owner. Ownership moves into `File`, which closes it exactly once.
    Ok(unsafe { File::from_raw_handle(handle) })
}

#[cfg(windows)]
fn directory_is_case_sensitive(directory: &File) -> std::io::Result<bool> {
    let mut info = FILE_CASE_SENSITIVE_INFO::default();
    // SAFETY: `directory` owns the live parent handle, `info` is aligned and writable for the
    // exact generated structure size, and no pointer escapes the synchronous query.
    let result = unsafe {
        GetFileInformationByHandleEx(
            directory.as_raw_handle(),
            FileCaseSensitiveInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_CASE_SENSITIVE_INFO>())
                .expect("FILE_CASE_SENSITIVE_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info.Flags & FILE_CS_FLAG_CASE_SENSITIVE_DIR != 0)
}

#[cfg(all(windows, test))]
enum DirectoryCaseSensitivityFixture {
    Enabled,
    AccessDenied(&'static str),
}

#[cfg(all(windows, test))]
fn enable_directory_case_sensitivity(
    path: &Path,
) -> std::io::Result<DirectoryCaseSensitivityFixture> {
    let directory = match OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
    {
        Ok(directory) => directory,
        Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => {
            return Ok(DirectoryCaseSensitivityFixture::AccessDenied(
                "open-directory-for-write-attributes",
            ));
        }
        Err(error) => return Err(error),
    };
    let info = FILE_CASE_SENSITIVE_INFO {
        Flags: FILE_CS_FLAG_CASE_SENSITIVE_DIR,
    };
    // SAFETY: this test-only adapter contract uses a live handle to a disposable empty directory;
    // `info` has the exact generated layout and remains live for the synchronous call.
    let result = unsafe {
        SetFileInformationByHandle(
            directory.as_raw_handle(),
            FileCaseSensitiveInfo,
            (&raw const info).cast(),
            u32::try_from(size_of::<FILE_CASE_SENSITIVE_INFO>())
                .expect("FILE_CASE_SENSITIVE_INFO size fits u32"),
        )
    };
    if result == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) {
            return Ok(DirectoryCaseSensitivityFixture::AccessDenied(
                "set-file-case-sensitive-info",
            ));
        }
        return Err(error);
    }
    if directory_is_case_sensitive(&directory)? {
        Ok(DirectoryCaseSensitivityFixture::Enabled)
    } else {
        Err(std::io::Error::other(
            "The directory accepted FileCaseSensitiveInfo but did not retain it",
        ))
    }
}

#[cfg(windows)]
fn open_validated_root_relative_source_file(
    configured_root: &Path,
    persisted_root_proof: &WindowsDirectoryRootProof,
    relative_path: &Path,
    metadata_handle: &File,
    evidence: AttributeTagEvidence,
) -> std::io::Result<File> {
    validate_source_file_info(&evidence)?;
    if evidence.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::other(
            "Recovery media access does not follow a terminal reparse point",
        ));
    }
    let expected_identity = raw_file_id_info(metadata_handle)?;
    let (publication_root, _) = WindowsDirectoryRootProof::open_configured(configured_root, false)?;
    if publication_root.identity != persisted_root_proof.identity
        || publication_root.volume_serial != persisted_root_proof.volume_serial
    {
        return Err(std::io::Error::other(
            "The configured source root changed identity before data access",
        ));
    }
    #[cfg(test)]
    PINNED_SOURCE_ACCESS_UPGRADE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    let data_handle = open_root_relative_data_handle(&publication_root.handle, relative_path)?;
    let data_info = file_attribute_tag_info_from_handle(&data_handle)?;
    validate_source_file_info(&AttributeTagEvidence {
        attributes: data_info.FileAttributes,
        reparse_tag: data_info.ReparseTag,
    })?;
    let data_identity = raw_file_id_info(&data_handle)?;
    if data_info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || data_identity.VolumeSerialNumber != expected_identity.VolumeSerialNumber
        || data_identity.FileId.Identifier != expected_identity.FileId.Identifier
        || data_identity.VolumeSerialNumber != publication_root.volume_serial
    {
        return Err(std::io::Error::other(
            "The pinned source object changed identity or left the selected root before data access",
        ));
    }
    validate_handle_within_root(&data_handle, &publication_root)?;
    #[cfg(test)]
    PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    #[cfg(test)]
    record_source_content_open(configured_root);
    Ok(data_handle)
}

#[cfg(windows)]
fn open_publication_namespace_guard(path: &Path) -> std::io::Result<File> {
    let desired_access = FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
    let share_access = FILE_SHARE_READ | FILE_SHARE_WRITE;
    let flags = FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT;
    #[cfg(test)]
    PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| {
        facts.borrow_mut().push(PublicationNamespaceGuardOpenFacts {
            path: path.to_path_buf(),
            desired_access,
            share_access,
            flags,
        });
    });
    OpenOptions::new()
        .access_mode(desired_access)
        .share_mode(share_access)
        .custom_flags(flags)
        .open(path)
}

#[cfg(windows)]
fn open_publication_namespace_child_guard(
    parent: &File,
    component: &std::ffi::OsStr,
    _path: &Path,
    parent_is_case_sensitive: bool,
) -> std::io::Result<File> {
    #[cfg(test)]
    PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| {
        facts.borrow_mut().push(PublicationNamespaceGuardOpenFacts {
            path: _path.to_path_buf(),
            desired_access: FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            share_access: FILE_SHARE_READ | FILE_SHARE_WRITE,
            flags: FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        });
    });
    nt_create_root_relative_handle_with_case_semantics(
        parent,
        component,
        FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        Some(true),
        false,
        false,
        parent_is_case_sensitive,
    )
}

#[cfg(windows)]
fn require_publication_namespace_acl_boundary(
    path: &Path,
    parent_path: &Path,
) -> std::io::Result<()> {
    for (access, label) in [
        (DELETE, "delete the namespace component"),
        (WRITE_DAC, "rewrite the namespace component DACL"),
        (WRITE_OWNER, "take ownership of the namespace component"),
    ] {
        require_absolute_access_denied(path, access, label)?;
    }
    for (access, label) in [
        (FILE_DELETE_CHILD, "delete a child through the parent"),
        (WRITE_DAC, "rewrite the parent DACL"),
        (WRITE_OWNER, "take ownership of the parent"),
    ] {
        require_absolute_access_denied(parent_path, access, label)?;
    }
    Ok(())
}

#[cfg(windows)]
fn require_absolute_access_denied(
    path: &Path,
    desired_access: u32,
    label: &str,
) -> std::io::Result<()> {
    match OpenOptions::new()
        .access_mode(desired_access)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
    {
        Err(error) if publication_namespace_access_is_denied(&error) => Ok(()),
        Err(error) => Err(std::io::Error::new(
            error.kind(),
            format!("Could not prove that the current token cannot {label} at {path:?}: {error}"),
        )),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "The current token can {label} at {path:?}; the ACL boundary does not protect the configured namespace"
            ),
        )),
    }
}

#[cfg(windows)]
fn publication_namespace_access_is_denied(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
}

#[cfg(windows)]
fn open_configured_root_handle(path: &Path, share_root_delete: bool) -> std::io::Result<File> {
    let desired_access = FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE;
    let share_access = FILE_SHARE_READ
        | FILE_SHARE_WRITE
        | if share_root_delete {
            FILE_SHARE_DELETE
        } else {
            0
        };
    let flags =
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT;
    #[cfg(test)]
    CONFIGURED_ROOT_OPENS.with(|facts| {
        facts.borrow_mut().push(ConfiguredRootOpenFacts {
            path: path.to_path_buf(),
            desired_access,
            share_access,
            flags,
        });
    });
    #[cfg(test)]
    {
        let mut counts = CONFIGURED_ROOT_OPEN_COUNTS
            .lock()
            .expect("configured-root open instrumentation");
        let count = counts
            .entry((path.to_path_buf(), share_root_delete))
            .or_insert(0);
        *count = count.saturating_add(1);
    }
    OpenOptions::new()
        .access_mode(desired_access)
        .share_mode(share_access)
        .custom_flags(flags)
        .open(path)
}

#[cfg(windows)]
impl WindowsDirectoryRootProof {
    fn identity_handle(&self) -> &File {
        self.metadata_handle.as_ref().unwrap_or(&self.handle)
    }

    fn try_clone_capability(&self) -> std::io::Result<Self> {
        Ok(Self {
            handle: self.handle.try_clone()?,
            metadata_handle: self
                .metadata_handle
                .as_ref()
                .map(File::try_clone)
                .transpose()?,
            identity: self.identity.clone(),
            volume_serial: self.volume_serial,
        })
    }

    fn open_configured(path: &Path, share_root_delete: bool) -> std::io::Result<(Self, PathBuf)> {
        let configured = open_configured_root_handle(path, share_root_delete).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("Could not open the configured root path: {error}"),
            )
        })?;
        let configured_info = file_attribute_tag_info_from_handle(&configured)?;
        if configured_info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            || configured_info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(std::io::Error::other(
                "The configured source root is not a no-follow directory handle",
            ));
        }
        let final_path = final_path_from_handle(&configured)?;
        let raw_identity = raw_file_id_info(&configured)?;
        let identity = file_identity_from_handle(&configured)?.ok_or_else(|| {
            std::io::Error::other("The resolved source root has no stable Windows file identity")
        })?;
        Ok((
            Self {
                handle: configured,
                metadata_handle: None,
                identity,
                volume_serial: raw_identity.VolumeSerialNumber,
            },
            final_path,
        ))
    }

    fn open_publication_namespace(
        path: &Path,
        expected_identity: Option<&FileIdentityEvidence>,
    ) -> std::io::Result<(Self, PathBuf, Vec<File>)> {
        let (configured_long_path, has_long_path_evidence) = match windows_long_dos_path(path) {
            Ok(path) => (path, true),
            Err(error) if publication_namespace_access_is_denied(&error) => {
                // A restricted ordinary-user token can traverse a protected profile prefix while
                // GetLongPathNameW is denied. The component guards, no-reparse checks, and final
                // durable root identity remain authoritative in that case.
                (path.to_path_buf(), false)
            }
            Err(error) => {
                return Err(std::io::Error::new(
                    error.kind(),
                    format!("Could not normalize the configured publication path: {error}"),
                ));
            }
        };
        let (volume_root, components) = publication_namespace_path(&configured_long_path)?;
        let volume = OpenOptions::new()
            .access_mode(FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&volume_root)
            .map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!("Could not pin the configured volume directory namespace: {error}"),
                )
            })?;
        validate_publication_namespace_directory(&volume, None).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("Could not validate the configured publication volume: {error}"),
            )
        })?;
        let volume_serial = raw_file_id_info(&volume)
            .map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!("Could not identify the configured publication volume: {error}"),
                )
            })?
            .VolumeSerialNumber;
        let mut ancestors = Vec::with_capacity(components.len().saturating_add(1));
        let mut parent_guard = Some(volume);
        let mut prefix_path = volume_root;
        let mut parent_path = prefix_path.clone();
        for component in components {
            prefix_path.push(&component);
            validate_publication_namespace_path(&prefix_path).map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!(
                        "Could not validate configured namespace component {:?}: {error}",
                        prefix_path
                    ),
                )
            })?;
            let opened = if let Some(parent) = parent_guard.as_ref() {
                let parent_is_case_sensitive =
                    directory_is_case_sensitive(parent).map_err(|error| {
                        std::io::Error::new(
                            error.kind(),
                            format!("Could not query configured namespace case semantics: {error}"),
                        )
                    })?;
                open_publication_namespace_child_guard(
                    parent,
                    &component,
                    &prefix_path,
                    parent_is_case_sensitive,
                )
                .and_then(|child| {
                    validate_publication_namespace_directory(&child, Some(volume_serial))?;
                    validate_publication_namespace_child(
                        parent,
                        &child,
                        &component,
                        parent_is_case_sensitive,
                    )?;
                    Ok(child)
                })
            } else {
                open_publication_namespace_guard(&prefix_path).and_then(|child| {
                    validate_publication_namespace_directory(&child, Some(volume_serial))?;
                    if has_long_path_evidence {
                        validate_publication_namespace_handle_path(&child, &prefix_path)?;
                    }
                    Ok(child)
                })
            };
            let child_guard = match opened {
                Ok(child_guard) => Some(child_guard),
                Err(error) if publication_namespace_access_is_denied(&error) => {
                    require_publication_namespace_acl_boundary(&prefix_path, &parent_path)
                        .map_err(|error| {
                            std::io::Error::new(
                                error.kind(),
                                format!(
                                    "Could not establish the ACL-protected publication boundary at {:?}: {error}",
                                    prefix_path
                                ),
                            )
                        })?;
                    None
                }
                Err(error) => {
                    return Err(std::io::Error::new(
                        error.kind(),
                        format!(
                            "Could not pin configured namespace prefix {:?} with no delete sharing: {error}",
                            prefix_path
                        ),
                    ));
                }
            };
            if let Some(parent) = parent_guard.take() {
                ancestors.push(parent);
            }
            parent_guard = child_guard;
            parent_path.clone_from(&prefix_path);
        }
        let guarded_root = parent_guard
            .as_ref()
            .map(|root| {
                Ok::<_, std::io::Error>((
                    file_identity_from_handle(root)?.ok_or_else(|| {
                        std::io::Error::other(
                            "The guarded publication root has no stable Windows file identity",
                        )
                    })?,
                    final_path_from_handle(root)?,
                ))
            })
            .transpose()?;
        if let Some(root) = parent_guard.take() {
            ancestors.push(root);
        }
        let (enumeration_root, final_path) = Self::open_configured(path, false)?;
        if has_long_path_evidence {
            validate_publication_namespace_handle_path(
                &enumeration_root.handle,
                &configured_long_path,
            )
            .map_err(|error| {
                std::io::Error::new(
                    error.kind(),
                    format!("Could not rebind the configured publication path: {error}"),
                )
            })?;
        }
        let raw_identity = raw_file_id_info(&enumeration_root.handle).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("Could not identify the configured publication root: {error}"),
            )
        })?;
        let identity = enumeration_root.identity.clone();
        if expected_identity.is_some_and(|expected| expected != &identity) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "The configured publication root does not match its durable identity",
            ));
        }
        if guarded_root.is_some_and(|(guarded_identity, guarded_path)| {
            guarded_identity != identity || guarded_path != final_path
        }) {
            return Err(std::io::Error::other(
                "The configured publication root changed while its namespace was pinned",
            ));
        }
        if enumeration_root.volume_serial != volume_serial
            || raw_identity.VolumeSerialNumber != volume_serial
        {
            return Err(std::io::Error::other(
                "The configured publication root changed while its namespace was pinned",
            ));
        }
        Ok((
            Self {
                handle: enumeration_root.handle,
                metadata_handle: None,
                identity,
                volume_serial: raw_identity.VolumeSerialNumber,
            },
            final_path,
            ancestors,
        ))
    }
}

#[cfg(windows)]
fn windows_long_dos_path(path: &Path) -> std::io::Result<PathBuf> {
    let mut source = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit == u16::from(b'/') {
                u16::from(b'\\')
            } else {
                unit
            }
        })
        .collect::<Vec<_>>();
    source.push(0);
    let mut buffer = vec![0_u16; source.len().max(512)];
    loop {
        // SAFETY: `source` is a terminated UTF-16 path and `buffer` is writable for its exact
        // capacity. GetLongPathNameW retains neither pointer.
        let length = unsafe {
            GetLongPathNameW(
                source.as_ptr(),
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len())
                    .map_err(|_| std::io::Error::other("The configured path is too long"))?,
            )
        };
        if length == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let length = usize::try_from(length)
            .map_err(|_| std::io::Error::other("The configured path is too long"))?;
        if length < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..length])));
        }
        buffer.resize(length.saturating_add(1), 0);
    }
}

#[cfg(windows)]
fn publication_namespace_path(path: &Path) -> std::io::Result<(PathBuf, Vec<OsString>)> {
    let mut components = path.components();
    let prefix = components.next().ok_or_else(|| {
        std::io::Error::other("The publication namespace path has no local volume prefix")
    })?;
    let drive = match prefix {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => {
                return Err(std::io::Error::other(
                    "The publication namespace must be on a local drive-letter volume",
                ));
            }
        },
        _ => {
            return Err(std::io::Error::other(
                "The publication namespace path has no local volume prefix",
            ));
        }
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(std::io::Error::other(
            "The publication namespace path is not absolute",
        ));
    }
    let mut relative_components = Vec::new();
    for component in components {
        match component {
            Component::Normal(value) => relative_components.push(value.to_os_string()),
            Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_)
            | Component::RootDir => {
                return Err(std::io::Error::other(
                    "The publication namespace path is not normalized",
                ));
            }
        }
    }
    if relative_components.is_empty() {
        return Err(std::io::Error::other(
            "A volume root cannot be configured as a library publication namespace",
        ));
    }
    let volume_root = PathBuf::from(format!("{}:\\", char::from(drive)));
    Ok((volume_root, relative_components))
}

#[cfg(windows)]
fn validate_publication_namespace_directory(
    directory: &File,
    expected_volume_serial: Option<u64>,
) -> std::io::Result<()> {
    let attributes = file_attribute_tag_info_from_handle(directory)
        .map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("Could not read configured namespace attributes: {error}"),
            )
        })?
        .FileAttributes;
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata_placeholder_state_from_attributes(attributes)
            != MetadataInventoryPlaceholderState::Available
    {
        return Err(std::io::Error::other(
            "The configured publication namespace contains an unavailable or reparse directory",
        ));
    }
    let identity = raw_file_id_info(directory).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Could not read configured namespace identity: {error}"),
        )
    })?;
    if expected_volume_serial.is_some_and(|expected| expected != identity.VolumeSerialNumber) {
        return Err(std::io::Error::other(
            "The configured publication namespace crossed a volume boundary",
        ));
    }
    if expected_volume_serial.is_none() {
        let mut filesystem_name = [0_u16; 32];
        // SAFETY: the live volume-directory handle remains owned for the call, optional output
        // pointers are null, and the writable name buffer has its exact element capacity.
        let result = unsafe {
            GetVolumeInformationByHandleW(
                directory.as_raw_handle(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                filesystem_name.as_mut_ptr(),
                u32::try_from(filesystem_name.len()).expect("filesystem name capacity fits u32"),
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let name_length = filesystem_name
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(filesystem_name.len());
        let filesystem = OsString::from_wide(&filesystem_name[..name_length]);
        if !windows_ordinal_equals(&filesystem, std::ffi::OsStr::new("NTFS"), true)? {
            return Err(std::io::Error::other(
                "The configured publication namespace is not on local NTFS",
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn validate_publication_namespace_path(path: &Path) -> std::io::Result<()> {
    let metadata = path.symlink_metadata()?;
    let attributes = metadata.file_attributes();
    if attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata_placeholder_state_from_attributes(attributes)
            != MetadataInventoryPlaceholderState::Available
    {
        return Err(std::io::Error::other(
            "The configured publication namespace contains an unavailable or reparse directory",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_publication_namespace_child(
    parent: &File,
    child: &File,
    component: &std::ffi::OsStr,
    parent_is_case_sensitive: bool,
) -> std::io::Result<()> {
    let parent_path = windows_path_parts(&final_path_from_handle(parent)?)?;
    let child_path = windows_path_parts(&final_path_from_handle(child)?)?;
    if !windows_ordinal_equals(&parent_path.anchor, &child_path.anchor, true)?
        || child_path.components.len() != parent_path.components.len().saturating_add(1)
    {
        return Err(std::io::Error::other(
            "The configured publication namespace escaped its pinned parent",
        ));
    }
    for (expected, actual) in parent_path
        .components
        .iter()
        .zip(child_path.components.iter())
    {
        if !windows_ordinal_equals(expected, actual, false)? {
            return Err(std::io::Error::other(
                "The configured publication namespace changed below its pinned parent",
            ));
        }
    }
    let Some(actual_component) = child_path.components.last() else {
        return Err(std::io::Error::other(
            "The configured publication namespace lost its terminal component",
        ));
    };
    if !windows_ordinal_equals(component, actual_component, !parent_is_case_sensitive)? {
        return Err(std::io::Error::other(
            "The configured publication namespace component changed identity",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_publication_namespace_handle_path(
    directory: &File,
    expected_path: &Path,
) -> std::io::Result<()> {
    let expected = windows_path_parts(expected_path)?;
    let actual = windows_path_parts(&final_path_from_handle(directory)?)?;
    if !windows_ordinal_equals(&expected.anchor, &actual.anchor, true)?
        || expected.components.len() != actual.components.len()
    {
        return Err(std::io::Error::other(
            "The configured publication namespace handle resolved to a different path",
        ));
    }
    for (expected, actual) in expected.components.iter().zip(actual.components.iter()) {
        if !windows_ordinal_equals(expected, actual, false)? {
            return Err(std::io::Error::other(
                "The configured publication namespace handle resolved to a different path",
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn file_attribute_tag_info_from_handle(file: &File) -> std::io::Result<FILE_ATTRIBUTE_TAG_INFO> {
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: ADR 0023 fixes this no-follow/no-recall query contract: the live file owns the
    // handle, `info` is aligned and writable, and the exact structure size is passed to Win32.
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .expect("FILE_ATTRIBUTE_TAG_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info)
}

#[cfg(windows)]
fn exact_attribute_tag_evidence(
    path: &Path,
    file: &File,
    known_attributes: u32,
) -> std::io::Result<AttributeTagEvidence> {
    let handle_info = file_attribute_tag_info_from_handle(file)?;
    let enumeration_info = exact_directory_entry_info(path)?;
    combine_attribute_tag_evidence(known_attributes, &handle_info, &enumeration_info)
}

#[cfg(windows)]
fn combine_attribute_tag_evidence(
    known_attributes: u32,
    handle_info: &FILE_ATTRIBUTE_TAG_INFO,
    enumeration_info: &WIN32_FIND_DATAW,
) -> std::io::Result<AttributeTagEvidence> {
    let attributes =
        known_attributes | handle_info.FileAttributes | enumeration_info.dwFileAttributes;
    let enumeration_tag = if has_reparse_point_attribute(enumeration_info.dwFileAttributes) {
        enumeration_info.dwReserved0
    } else {
        0
    };
    if handle_info.ReparseTag != 0
        && enumeration_tag != 0
        && handle_info.ReparseTag != enumeration_tag
    {
        return Err(std::io::Error::other(
            "The source reparse identity changed during classification",
        ));
    }
    Ok(AttributeTagEvidence {
        attributes,
        reparse_tag: if handle_info.ReparseTag != 0 {
            handle_info.ReparseTag
        } else {
            enumeration_tag
        },
    })
}

#[cfg(windows)]
fn exact_directory_entry_info(path: &Path) -> std::io::Result<WIN32_FIND_DATAW> {
    let wide_path = windows_extended_path(path);
    let mut info = WIN32_FIND_DATAW::default();
    // SAFETY: ADR 0023 fixes this exact-name enumeration contract: `wide_path` is a terminated
    // UTF-16 path, `info` is aligned and writable, and the returned search handle is closed once.
    let handle = unsafe { FindFirstFileW(wide_path.as_ptr(), &raw mut info) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `handle` is the live search handle returned by the successful call above and is not
    // used again after this close.
    let close_result = unsafe { FindClose(handle) };
    if close_result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info)
}

#[cfg(windows)]
fn windows_extended_path(path: &Path) -> Vec<u16> {
    let separator = u16::from(b'\\');
    let mut path_units = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit == u16::from(b'/') {
                separator
            } else {
                unit
            }
        })
        .collect::<Vec<_>>();
    let is_device_path =
        path_units.starts_with(&[separator, separator, u16::from(b'?'), separator])
            || path_units.starts_with(&[separator, separator, u16::from(b'.'), separator]);
    if path.is_absolute() && !is_device_path {
        let mut extended = std::vec::Vec::with_capacity(4);
        extended.extend_from_slice(&[separator, separator, u16::from(b'?'), separator]);
        if path_units.starts_with(&[separator, separator]) {
            extended.extend("UNC\\".encode_utf16());
            extended.extend_from_slice(&path_units[2..]);
        } else {
            extended.append(&mut path_units);
        }
        path_units = extended;
    }
    path_units.push(0);
    path_units
}

#[cfg(not(windows))]
fn file_identity(_path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    Ok(None)
}

#[cfg(windows)]
pub(crate) fn open_source_file(path: &Path, source_root: &Path) -> std::io::Result<File> {
    open_source_file_with_hook(path, source_root, || {})
}

#[cfg(windows)]
fn open_source_file_with_hook(
    path: &Path,
    source_root: &Path,
    after_validation: impl FnOnce(),
) -> std::io::Result<File> {
    let (root_proof, _) = WindowsDirectoryRootProof::open_configured(source_root, true)?;
    open_source_file_with_root_proof(path, source_root, &root_proof, after_validation)
}

#[cfg(windows)]
fn open_source_file_with_root_proof(
    path: &Path,
    _source_root: &Path,
    root_proof: &WindowsDirectoryRootProof,
    after_validation: impl FnOnce(),
) -> std::io::Result<File> {
    let validation = open_validation_handle(path)?;
    let validation_evidence = exact_attribute_tag_evidence(path, &validation, 0)?;
    validate_source_file_info(&validation_evidence)?;
    let expected_identity = file_identity_from_handle(&validation)?;
    after_validation();
    let source = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_NO_RECALL)
        .open(path)?;
    let source_info = file_attribute_tag_info_from_handle(&source)?;
    validate_source_file_info(&AttributeTagEvidence {
        attributes: source_info.FileAttributes,
        reparse_tag: source_info.ReparseTag,
    })?;
    if file_identity_from_handle(&source)? != expected_identity {
        return Err(std::io::Error::other(
            "The source path changed while it was being opened",
        ));
    }
    validate_handle_within_root(&source, root_proof)?;
    #[cfg(test)]
    record_source_content_open(_source_root);
    Ok(source)
}

#[cfg(windows)]
fn validate_handle_within_root(
    source: &File,
    root_proof: &WindowsDirectoryRootProof,
) -> std::io::Result<()> {
    let candidate = windows_path_parts(&final_path_from_handle(source)?)?;
    let root = windows_path_parts(&final_path_from_handle(&root_proof.handle)?)?;
    if !windows_ordinal_equals(&candidate.anchor, &root.anchor, true)?
        || candidate.components.len() <= root.components.len()
    {
        return Err(std::io::Error::other(
            "The source handle resolved outside the selected library root",
        ));
    }

    let mut candidate_root_path = PathBuf::from(&candidate.anchor);
    for component in candidate.components.iter().take(root.components.len()) {
        candidate_root_path.push(component);
    }
    let candidate_root = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT,
        )
        .open(candidate_root_path)?;
    let attributes = file_attribute_tag_info_from_handle(&candidate_root)?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || file_identity_from_handle(&candidate_root)?.as_ref() != Some(&root_proof.identity)
    {
        return Err(std::io::Error::other(
            "The source handle resolved outside the selected library root",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn final_path_from_handle(file: &File) -> std::io::Result<PathBuf> {
    let mut buffer = vec![0_u16; 512];
    loop {
        let capacity = u32::try_from(buffer.len())
            .map_err(|_| std::io::Error::other("The resolved source path is too long"))?;
        // SAFETY: ADR 0023 requires final containment to be derived from the live source handle.
        // The buffer is writable for `capacity` UTF-16 code units and remains alive for the call.
        let length = unsafe {
            GetFinalPathNameByHandleW(file.as_raw_handle(), buffer.as_mut_ptr(), capacity, 0)
        };
        if length == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let length = usize::try_from(length)
            .map_err(|_| std::io::Error::other("The resolved source path is too long"))?;
        if length < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..length])));
        }
        buffer.resize(length.saturating_add(1), 0);
    }
}

#[cfg(windows)]
struct WindowsPathParts {
    anchor: OsString,
    components: Vec<OsString>,
}

#[cfg(windows)]
fn windows_path_parts(path: &Path) -> std::io::Result<WindowsPathParts> {
    let visible = PathBuf::from(user_visible_path(&path.to_string_lossy()));
    let mut anchor = OsString::new();
    let mut components = Vec::new();
    for component in visible.components() {
        match component {
            Component::Prefix(prefix) => anchor.push(prefix.as_os_str()),
            Component::RootDir => anchor.push(component.as_os_str()),
            Component::Normal(value) => components.push(value.to_os_string()),
            Component::CurDir | Component::ParentDir => {
                return Err(std::io::Error::other(
                    "A resolved Windows handle path was not normalized",
                ));
            }
        }
    }
    if anchor.is_empty() {
        return Err(std::io::Error::other(
            "A resolved Windows handle path had no volume anchor",
        ));
    }
    Ok(WindowsPathParts { anchor, components })
}

#[cfg(windows)]
fn windows_ordinal_equals(
    left: &std::ffi::OsStr,
    right: &std::ffi::OsStr,
    ignore_case: bool,
) -> std::io::Result<bool> {
    let left = left.encode_wide().collect::<Vec<_>>();
    let right = right.encode_wide().collect::<Vec<_>>();
    let left_len = i32::try_from(left.len())
        .map_err(|_| std::io::Error::other("The Windows path component is too long"))?;
    let right_len = i32::try_from(right.len())
        .map_err(|_| std::io::Error::other("The Windows path component is too long"))?;
    // SAFETY: both UTF-16 buffers remain live for the call and the exact element counts are
    // supplied. CompareStringOrdinal retains neither pointer.
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left_len,
            right.as_ptr(),
            right_len,
            i32::from(ignore_case),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(result == CSTR_EQUAL)
}

#[cfg(all(windows, test))]
pub(crate) fn canonical_source_root_path(path: &Path) -> std::io::Result<PathBuf> {
    WindowsDirectoryRootProof::open_configured(path, true).map(|(_, final_path)| final_path)
}

#[cfg(windows)]
fn validate_source_file_info(info: &AttributeTagEvidence) -> std::io::Result<()> {
    let attributes = info.attributes;
    let placeholder_state = metadata_placeholder_state_from_attributes(attributes);
    if placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(std::io::Error::other(
            "The cloud source content is not locally available",
        ));
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        let cloud_state = cloud_placeholder_state_from_attribute_tag(attributes, info.reparse_tag)?;
        if cloud_state & CF_PLACEHOLDER_STATE_PLACEHOLDER == 0 {
            return Err(std::io::Error::other(
                "The source path is an unsupported reparse point",
            ));
        }
        if cloud_state & (CF_PLACEHOLDER_STATE_PARTIAL | CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK)
            != 0
        {
            return Err(std::io::Error::other(
                "The cloud source content is not locally available",
            ));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn open_source_file(path: &Path, _source_root: &Path) -> std::io::Result<File> {
    File::open(path)
}

#[cfg(not(windows))]
pub(crate) fn canonical_source_root_path(path: &Path) -> std::io::Result<PathBuf> {
    path.canonicalize()
}

#[cfg(windows)]
fn cloud_placeholder_state_from_attribute_tag(
    attributes: u32,
    reparse_tag: u32,
) -> std::io::Result<u32> {
    // SAFETY: ADR 0023 limits this value-only Cloud Files call to initialized file attributes and
    // a reparse tag returned by `FileAttributeTagInfo` for the same no-follow, no-recall handle.
    let state = unsafe { CfGetPlaceholderStateFromAttributeTag(attributes, reparse_tag) };
    if state == CF_PLACEHOLDER_STATE_INVALID {
        Err(std::io::Error::other(
            "Cloud Files rejected the file attributes and reparse tag",
        ))
    } else {
        Ok(state)
    }
}

fn modified_unix_ms(metadata: &Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn created_unix_ms(metadata: &Metadata) -> Option<i64> {
    metadata
        .created()
        .ok()
        .and_then(|created| created.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn path_text(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

fn relative_path_text(path: impl AsRef<Path>) -> String {
    path_text(path).replace('\\', "/")
}

pub(crate) fn user_visible_path(path: &str) -> String {
    const DEVICE_PREFIX: &str = "\\\\?\\";
    const UNC_DEVICE_PREFIX: &str = "\\\\?\\UNC\\";
    if path
        .get(..UNC_DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(UNC_DEVICE_PREFIX))
    {
        return format!("\\\\{}", &path[UNC_DEVICE_PREFIX.len()..]);
    }
    if path
        .get(..DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(DEVICE_PREFIX))
    {
        return path[DEVICE_PREFIX.len()..].to_owned();
    }
    path.to_owned()
}

#[cfg(all(windows, test))]
fn has_cloud_placeholder_attribute(attributes: u32) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };

    attributes & FILE_ATTRIBUTE_OFFLINE != 0
        || attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0
        || attributes & FILE_ATTRIBUTE_RECALL_ON_OPEN != 0
}

#[cfg(windows)]
fn metadata_placeholder_state(metadata: &Metadata) -> MetadataInventoryPlaceholderState {
    metadata_placeholder_state_from_attributes(metadata.file_attributes())
}

#[cfg(windows)]
fn metadata_placeholder_state_from_attributes(
    attributes: u32,
) -> MetadataInventoryPlaceholderState {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };

    if attributes & FILE_ATTRIBUTE_OFFLINE != 0 {
        MetadataInventoryPlaceholderState::Offline
    } else if attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0 {
        MetadataInventoryPlaceholderState::RecallOnDataAccess
    } else if attributes & FILE_ATTRIBUTE_RECALL_ON_OPEN != 0 {
        MetadataInventoryPlaceholderState::RecallOnOpen
    } else {
        MetadataInventoryPlaceholderState::Available
    }
}

#[cfg(not(windows))]
fn metadata_placeholder_state(_metadata: &Metadata) -> MetadataInventoryPlaceholderState {
    MetadataInventoryPlaceholderState::Available
}

#[cfg(test)]
mod tests {
    mod availability_module_topology;

    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::fs;
    #[cfg(windows)]
    use std::io::Read;
    #[cfg(windows)]
    use std::thread;
    #[cfg(windows)]
    use std::time::{Duration, Instant};

    use quote::ToTokens;
    use syn::visit::{self, Visit};
    use syn::{Attribute, ExprCall, ExprMacro, Item, ItemFn, ItemImpl, Type, UseTree};
    use tempfile::{tempdir, tempdir_in};

    use super::*;

    #[cfg(windows)]
    fn rename_disposable_directory(source: &Path, destination: &Path, label: &str) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match fs::rename(source, destination) {
                Ok(()) => return,
                Err(error)
                    if matches!(error.raw_os_error(), Some(5) | Some(32))
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{label}: {error}"),
            }
        }
    }

    #[test]
    fn publication_guard_api_is_explicit_and_cursor_bound() {
        trait AmbiguousIfDeref<Marker> {
            fn assert_not_deref() {}
        }
        impl<T: ?Sized> AmbiguousIfDeref<()> for T {}
        struct DerefMarker;
        impl<T> AmbiguousIfDeref<DerefMarker> for T where T: ?Sized + std::ops::Deref<Target = FileDiscovery>
        {}

        let _ = <PublicationGuardedFileDiscovery as AmbiguousIfDeref<_>>::assert_not_deref;
        let _: for<'guard> fn(
            &'guard PublicationGuardedFileDiscovery,
            &str,
        ) -> Result<PublicationGuardedFileVisits<'guard>, ScanIssue> =
            PublicationGuardedFileDiscovery::file_visits_in_directory;
        let _: fn(
            &PublicationGuardedFileDiscovery,
            &str,
        ) -> Result<PublicationGuardedMetadataInventoryEntries, ScanIssue> =
            PublicationGuardedFileDiscovery::streaming_metadata_inventory_entries_in_directory;
    }

    #[test]
    fn user_visible_path_hides_windows_device_prefixes() {
        assert_eq!(
            user_visible_path(r"\\?\G:\Pictures\sample.png"),
            r"G:\Pictures\sample.png"
        );
        assert_eq!(
            user_visible_path(r"\\?\UNC\server\share\sample.png"),
            r"\\server\share\sample.png"
        );
        assert_eq!(
            user_visible_path(r"C:\Pictures\sample.png"),
            r"C:\Pictures\sample.png"
        );
    }

    #[test]
    fn discovery_emits_platform_independent_relative_paths() {
        let root = tempdir().expect("source root");
        let nested = root.path().join("album");
        fs::create_dir(&nested).expect("nested directory");
        fs::write(nested.join("photo.png"), b"\x89PNG\r\n\x1a\nfixture").expect("fixture image");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
        let relative_path = discovery
            .entry_paths_in_directory("album")
            .expect("directory entries")
            .next()
            .expect("nested image");
        let visit = discovery.visit_relative_path(&relative_path);

        assert_eq!(relative_path, "album/photo.png");
        assert_eq!(visit.relative_path, "album/photo.png");
    }

    #[test]
    fn iso_bmff_video_header_is_not_classified_as_an_image() {
        let root = tempdir().expect("source root");
        let video = root.path().join("clip.mp4");
        fs::write(&video, b"\0\0\0\x18ftypmp42\0\0\0\0").expect("video header");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");

        let visit = discovery.visit_relative_path("clip.mp4");

        assert!(matches!(
            visit.outcome,
            FileVisitOutcome::TerminalMedia {
                issue,
                report_issue: false,
                ..
            } if issue.code == "media_type_unsupported"
        ));
    }

    #[test]
    fn supported_image_magic_remains_available_for_unknown_extensions() {
        let root = tempdir().expect("source root");
        fs::write(root.path().join("image.data"), b"\x89PNG\r\n\x1a\nfixture")
            .expect("image header");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");

        match discovery.visit_relative_path("image.data").outcome {
            FileVisitOutcome::File(_) => {}
            FileVisitOutcome::Issue(issue) | FileVisitOutcome::RetryableFile { issue, .. } => {
                panic!(
                    "supported image magic was unreadable: {}: {}",
                    issue.code, issue.message
                )
            }
            FileVisitOutcome::TerminalMedia { issue, .. } => panic!(
                "supported image magic was terminal: {}: {}",
                issue.code, issue.message
            ),
            FileVisitOutcome::Directory => {
                panic!("supported image magic was classified as a directory")
            }
            FileVisitOutcome::Ignored => {
                panic!("supported image magic was classified as ignored")
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn recognizes_every_windows_cloud_recall_attribute() {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        };

        assert!(has_cloud_placeholder_attribute(FILE_ATTRIBUTE_OFFLINE));
        assert!(has_cloud_placeholder_attribute(
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        ));
        assert!(has_cloud_placeholder_attribute(
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        ));
        assert!(!has_cloud_placeholder_attribute(0));
    }

    #[cfg(windows)]
    #[test]
    fn recognizes_windows_reparse_points() {
        assert!(has_reparse_point_attribute(FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(!has_reparse_point_attribute(0));
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn intermediate_filesystem_link_cannot_escape_the_discovery_root() {
        let root = tempdir().expect("source root");
        let outside = tempdir().expect("outside directory");
        fs::write(outside.path().join("outside.png"), b"outside bytes").expect("outside fixture");
        let link = root.path().join("linked");
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_dir(outside.path(), &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("create directory link: {error}");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &link).expect("create directory link");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("file discovery");

        let visit = discovery.visit_relative_path("linked/outside.png");

        let FileVisitOutcome::Issue(issue) = visit.outcome else {
            panic!("an intermediate link must be rejected");
        };
        assert_eq!(
            issue.code, "source_path_outside_root",
            "unexpected root-relative open failure: {}",
            issue.message
        );

        #[cfg(windows)]
        {
            let error = match FileDiscovery::new(&link.to_string_lossy()) {
                Ok(_) => panic!("an explicitly selected reparse root must fail closed"),
                Err(error) => error,
            };
            assert_eq!(error.code, "root_identity_unavailable");
        }
        #[cfg(unix)]
        let explicitly_selected =
            FileDiscovery::new(&link.to_string_lossy()).expect("explicit linked root");
        #[cfg(unix)]
        assert!(matches!(
            explicitly_selected
                .visit_relative_path("outside.png")
                .outcome,
            FileVisitOutcome::File(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn offline_file_is_reported_before_image_content_is_opened() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let file_path = directory.path().join("offline.png");
        let original = b"this content must not be decoded";
        fs::write(&file_path, original).expect("fixture write");
        let set_offline = Command::new("attrib.exe")
            .arg("+O")
            .arg(&file_path)
            .status()
            .expect("attrib executable");
        assert!(set_offline.success());

        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("valid discovery root");
        let issues = discovery
            .entry_paths_in_directory("")
            .expect("directory entries")
            .map(|relative_path| discovery.visit_relative_path(&relative_path))
            .filter_map(|visit| match visit.outcome {
                FileVisitOutcome::Issue(issue) => Some(issue),
                _ => None,
            })
            .collect::<Vec<_>>();
        let inventory_entry = discovery
            .metadata_inventory_entry("offline.png")
            .expect("offline metadata inventory entry");
        let automatic_visit = discovery.visit_relative_path("offline.png");
        let source_root = canonical_source_root_path(directory.path()).expect("canonical root");
        let guarded_open = open_source_file(&file_path, &source_root);

        let clear_offline = Command::new("attrib.exe")
            .arg("-O")
            .arg(&file_path)
            .status()
            .expect("attrib executable");
        assert!(clear_offline.success());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "cloud_placeholder_skipped");
        assert!(guarded_open.is_err());
        assert!(matches!(
            automatic_visit.outcome,
            FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped"
        ));
        assert_eq!(
            inventory_entry.placeholder_state,
            MetadataInventoryPlaceholderState::Offline
        );
        assert_eq!(inventory_entry.kind, MetadataInventoryEntryKind::File);
        assert!(inventory_entry.file_identity.is_none());
        assert_eq!(fs::read(file_path).expect("fixture bytes"), original);
    }

    #[cfg(windows)]
    #[test]
    fn guarded_source_open_rejects_replacement_and_rechecks_availability() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.png");
        let source_root = canonical_source_root_path(directory.path()).expect("canonical root");
        fs::write(&source_path, b"source-bytes").expect("source fixture");
        let replaced = open_source_file_with_hook(&source_path, &source_root, || {
            fs::remove_file(&source_path).expect("remove original fixture");
            fs::write(&source_path, b"replacement").expect("replacement fixture");
        });
        assert!(replaced.is_err());

        fs::write(&source_path, b"source-bytes").expect("restore source fixture");

        let dehydrated = open_source_file_with_hook(&source_path, &source_root, || {
            let status = Command::new("attrib.exe")
                .arg("+O")
                .arg(&source_path)
                .status()
                .expect("attrib executable");
            assert!(status.success());
        });
        let clear_offline = Command::new("attrib.exe")
            .arg("-O")
            .arg(&source_path)
            .status()
            .expect("attrib executable");
        assert!(clear_offline.success());
        assert!(dehydrated.is_err());
    }

    #[cfg(windows)]
    #[test]
    fn guarded_source_open_rejects_an_ancestor_junction_swap() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("root");
        let outside = directory.path().join("outside");
        let album = root.join("album");
        fs::create_dir_all(&album).expect("source directory");
        fs::create_dir_all(&outside).expect("outside directory");
        fs::write(album.join("source.png"), b"inside").expect("inside source");
        fs::write(outside.join("source.png"), b"outside").expect("outside source");
        let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("file discovery");
        discovery
            .checked_existing_path(Path::new("album/source.png"))
            .expect("initial path is contained");

        fs::rename(&album, root.join("album-original")).expect("move original directory");
        let junction = Command::new("cmd.exe")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(&album)
            .arg(&outside)
            .status()
            .expect("junction command");
        assert!(junction.success());

        let opened = open_source_file(&album.join("source.png"), &discovery.canonical_root);
        fs::remove_dir(&album).expect("remove junction");

        assert!(opened.is_err());
    }

    #[cfg(windows)]
    #[test]
    fn handle_directory_open_enumerates_a_nested_nonreparse_directory() {
        let directory = tempdir().expect("temporary directory");
        let album = directory.path().join("album");
        fs::create_dir_all(&album).expect("nested directory");
        fs::write(album.join("inside.png"), b"inside").expect("nested source");
        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");

        let mut opened = discovery
            .streaming_checked_entry_paths_in_directory("album")
            .expect("root-relative nested directory");
        let entry = opened
            .next()
            .expect("nested entry")
            .expect("valid nested entry");
        assert_eq!(entry.relative_path, "album/inside.png");
        assert!(opened.next().is_none());
        assert!(opened.finish().expect("finish nested directory").is_some());
    }

    #[cfg(windows)]
    #[test]
    fn handle_directory_open_completes_an_empty_directory_without_a_false_failure() {
        let directory = tempdir().expect("temporary directory");
        let empty = directory.path().join("empty");
        fs::create_dir(&empty).expect("empty directory");
        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");

        let mut opened = discovery
            .streaming_checked_entry_paths_in_directory("empty")
            .expect("root-relative empty directory");

        assert!(opened.next().is_none());
        assert!(opened.finish().expect("finish empty directory").is_some());
    }

    #[cfg(windows)]
    #[test]
    fn handle_directory_open_rejects_an_ancestor_junction_swap_at_the_authorization_window() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("root");
        let outside = directory.path().join("outside");
        let album = root.join("album");
        fs::create_dir_all(&album).expect("source directory");
        fs::create_dir_all(&outside).expect("outside directory");
        fs::write(album.join("inside.png"), b"inside").expect("inside source");
        fs::write(outside.join("outside.png"), b"outside").expect("outside source");
        let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("file discovery");

        let opened =
            discovery.streaming_checked_entry_paths_in_directory_with_hook("album", || {
                fs::rename(&album, root.join("album-original")).expect("move authorized directory");
                let junction = Command::new("cmd.exe")
                    .arg("/C")
                    .arg("mklink")
                    .arg("/J")
                    .arg(&album)
                    .arg(&outside)
                    .status()
                    .expect("junction command");
                assert!(junction.success());
            });
        fs::remove_dir(&album).expect("remove junction");

        let issue = match opened {
            Ok(_) => panic!("the external junction target must not be enumerated"),
            Err(issue) => issue,
        };
        assert_eq!(
            issue.code, "source_path_outside_root",
            "unexpected root-relative open failure: {}",
            issue.message
        );
    }

    #[cfg(windows)]
    #[test]
    fn pinned_content_upgrade_rejects_an_ancestor_path_replacement() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("root");
        let outside = directory.path().join("outside");
        let album = root.join("album");
        fs::create_dir_all(&album).expect("source directory");
        fs::create_dir_all(&outside).expect("outside directory");
        fs::write(album.join("source.png"), b"authorized-object").expect("source content");
        fs::write(outside.join("source.png"), b"external-object").expect("external content");
        let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("file discovery");

        PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(|count| count.set(0));
        let error = discovery
            .open_pinned_source_file_with_hook("album/source.png", || {
                fs::rename(album.join("source.png"), root.join("authorized-source.png"))
                    .expect("move authorized object away from the ancestor");
                fs::remove_dir(&album).expect("remove emptied ancestor");
                let junction = Command::new("cmd.exe")
                    .arg("/C")
                    .arg("mklink")
                    .arg("/J")
                    .arg(&album)
                    .arg(&outside)
                    .status()
                    .expect("junction command");
                assert!(junction.success());
            })
            .expect_err("a replaced ancestor must invalidate the path-bound data upgrade");
        fs::remove_dir(&album).expect("remove replacement junction");

        assert!(
            error.to_string().contains("reparse")
                || error.to_string().contains("root-relative component"),
            "unexpected ancestor replacement failure: {error}"
        );
        assert_eq!(PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(Cell::get), 0);
        assert_eq!(
            fs::read(outside.join("source.png")).expect("external file"),
            b"external-object"
        );
    }

    #[cfg(windows)]
    #[test]
    fn pinned_content_upgrade_rejects_an_object_moved_outside_the_root() {
        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("root");
        let outside = directory.path().join("outside");
        let album = root.join("album");
        fs::create_dir_all(&album).expect("source directory");
        fs::create_dir_all(&outside).expect("outside directory");
        fs::write(album.join("source.png"), b"authorized-object").expect("source content");
        let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("file discovery");
        PINNED_SOURCE_ACCESS_UPGRADE_COUNT.with(|count| count.set(0));
        PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(|count| count.set(0));
        CONFIGURED_ROOT_OPENS.with(|facts| facts.borrow_mut().clear());
        LAST_ROOT_RELATIVE_METADATA_OPEN.with(|facts| facts.set(None));
        LAST_ROOT_RELATIVE_DATA_OPEN.with(|facts| facts.set(None));

        let error = discovery
            .open_pinned_source_file_with_hook("album/source.png", || {
                fs::rename(album.join("source.png"), outside.join("source.png"))
                    .expect("move source outside the selected root");
                fs::write(album.join("source.png"), b"replacement-object")
                    .expect("replacement source");
            })
            .expect_err("an object outside the pinned root must not become readable");

        assert_eq!(PINNED_SOURCE_ACCESS_UPGRADE_COUNT.with(Cell::get), 1);
        let metadata_open = LAST_ROOT_RELATIVE_METADATA_OPEN
            .with(Cell::get)
            .expect("root-relative metadata open facts");
        assert_eq!(
            metadata_open.desired_access,
            FILE_READ_ATTRIBUTES | SYNCHRONIZE
        );
        assert_eq!(
            metadata_open.share_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
        );
        assert!(metadata_open.ea_buffer_is_null);
        assert_eq!(metadata_open.ea_length, 0);
        assert_ne!(metadata_open.create_options & FILE_OPEN_REPARSE_POINT, 0);
        assert_eq!(
            metadata_open.create_options & FILE_OPEN_NO_RECALL_NATIVE,
            0,
            "metadata-only opening must not use the directory-incompatible native no-recall option"
        );
        let data_open = LAST_ROOT_RELATIVE_DATA_OPEN
            .with(Cell::get)
            .expect("root-relative data open facts");
        assert_ne!(
            data_open.desired_access & FILE_READ_DATA,
            0,
            "the terminal data handle must be the only content-capable open"
        );
        assert_eq!(data_open.share_access, FILE_SHARE_READ | FILE_SHARE_WRITE);
        assert_ne!(
            data_open.create_options & FILE_OPEN_NO_RECALL_NATIVE,
            0,
            "the terminal data open must carry native no-recall"
        );
        assert!(data_open.ea_buffer_is_null);
        assert_eq!(data_open.ea_length, 0);
        assert!(!data_open.object_name_is_absolute);
        assert!(!data_open.object_name_has_separator);
        assert!(
            CONFIGURED_ROOT_OPENS.with(|facts| facts.borrow().iter().all(|fact| {
                !windows_path_uses_device_namespace(&fact.path)
                    && fact.desired_access
                        == (FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
                    && fact.share_access == (FILE_SHARE_READ | FILE_SHARE_WRITE)
                    && fact.flags
                        == (FILE_FLAG_BACKUP_SEMANTICS
                            | FILE_FLAG_OPEN_NO_RECALL
                            | FILE_FLAG_OPEN_REPARSE_POINT)
            }))
        );
        assert!(error.to_string().contains("changed identity"));
        assert_eq!(
            PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(Cell::get),
            0,
            "an external object must never be authorized for a source-content read"
        );
        assert_eq!(
            fs::read(outside.join("source.png")).expect("moved source"),
            b"authorized-object"
        );
        assert_eq!(
            fs::read(album.join("source.png")).expect("replacement source"),
            b"replacement-object"
        );
    }

    #[cfg(windows)]
    #[test]
    fn root_lookup_obeys_the_real_parent_case_semantics() {
        let directory = tempdir_in(std::env::temp_dir())
            .expect("ordinary-user LocalAppData temporary directory");
        let parent = directory.path().join("sensitive-parent");
        fs::create_dir(&parent).expect("case parent");
        match enable_directory_case_sensitivity(&parent).expect("enable case semantics") {
            DirectoryCaseSensitivityFixture::Enabled => {}
            DirectoryCaseSensitivityFixture::AccessDenied(operation) => {
                eprintln!(
                    "skipped case-sensitive directory fixture: ordinary token was denied during {operation}"
                );
                return;
            }
        }
        let upper = parent.join("Photos");
        fs::create_dir(&upper).expect("upper-case sibling");
        fs::write(upper.join("upper.png"), b"upper").expect("upper fixture");

        let lower = parent.join("photos");
        fs::create_dir(&lower).expect("lower-case sibling");
        fs::write(lower.join("lower.png"), b"lower").expect("lower fixture");
        let nested = upper.join("NestedSensitive");
        fs::create_dir(&nested).expect("nested case-sensitive parent");
        match enable_directory_case_sensitivity(&nested).expect("enable nested case semantics") {
            DirectoryCaseSensitivityFixture::Enabled => {}
            DirectoryCaseSensitivityFixture::AccessDenied(operation) => {
                eprintln!(
                    "skipped nested case-sensitive directory fixture: ordinary token was denied during {operation}"
                );
                return;
            }
        }
        let nested_upper = nested.join("Album");
        let nested_lower = nested.join("album");
        fs::create_dir(&nested_upper).expect("nested upper-case sibling");
        fs::create_dir(&nested_lower).expect("nested lower-case sibling");
        fs::write(nested_upper.join("upper-nested.png"), b"nested-upper")
            .expect("nested upper fixture");
        fs::write(nested_lower.join("lower-nested.png"), b"nested-lower")
            .expect("nested lower fixture");
        let upper_discovery = FileDiscovery::new(&upper.to_string_lossy()).expect("upper root");
        let lower_discovery = FileDiscovery::new(&lower.to_string_lossy()).expect("lower root");

        assert_ne!(
            upper_discovery
                .metadata_inventory_root_identity()
                .expect("upper identity"),
            lower_discovery
                .metadata_inventory_root_identity()
                .expect("lower identity")
        );
        assert!(FileDiscovery::new(&parent.join("PHOTOS").to_string_lossy()).is_err());
        assert!(matches!(
            upper_discovery.visit_relative_path("upper.png").outcome,
            FileVisitOutcome::File(_)
        ));
        assert!(matches!(
            lower_discovery.visit_relative_path("lower.png").outcome,
            FileVisitOutcome::File(_)
        ));
        assert!(matches!(
            upper_discovery
                .visit_relative_path("NestedSensitive/Album/upper-nested.png")
                .outcome,
            FileVisitOutcome::File(_)
        ));
        assert!(matches!(
            upper_discovery
                .visit_relative_path("NestedSensitive/album/lower-nested.png")
                .outcome,
            FileVisitOutcome::File(_)
        ));
        assert!(matches!(
            upper_discovery
                .visit_relative_path("NestedSensitive/ALBUM/upper-nested.png")
                .outcome,
            FileVisitOutcome::Issue(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn publication_guard_rebinds_the_configured_path_and_pins_its_identity() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("long-path temporary directory");
        let root = directory.path().join("root");
        let moved = directory.path().join("moved-root");
        fs::create_dir(&root).expect("source root");
        let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("source discovery");
        let expected_identity = discovery
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");

        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &root.to_string_lossy(),
            &expected_identity,
        )
        .expect("unchanged configured root");
        assert!(
            fs::rename(&root, &moved).is_err(),
            "the publication guard must prevent a rename after its boundary pin"
        );
        drop(guard);

        rename_disposable_directory(&root, &moved, "rename after publication boundary");
        assert!(
            PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                &root.to_string_lossy(),
                &expected_identity,
            )
            .is_err(),
            "a missing configured path must fail closed"
        );
        fs::create_dir(&root).expect("replacement root");
        assert!(
            PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                &root.to_string_lossy(),
                &expected_identity,
            )
            .is_err(),
            "a replacement at the configured path must not inherit authority"
        );
    }

    #[cfg(windows)]
    #[test]
    fn preview_publication_guard_pins_file_root_and_mutable_ancestors_until_drop() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("ordinary-user temporary directory");
        let ancestor = directory.path().join("ancestor");
        let root = ancestor.join("root");
        let source = root.join("source.png");
        fs::create_dir_all(&root).expect("preview source root");
        fs::write(&source, b"preview-source").expect("preview source");
        let metadata = source.metadata().expect("preview source metadata");
        let (file_identity, source_revision) =
            file_source_evidence(&source).expect("preview source evidence");
        let root_identity = FileDiscovery::new(&root.to_string_lossy())
            .expect("source discovery")
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");
        let expected = ExpectedFileState {
            absolute_path: path_text(&source),
            file_size: metadata.len(),
            modified_unix_ms: modified_unix_ms(&metadata),
            file_identity,
            source_revision: Some(source_revision),
        };

        let guard = open_preview_publication_guard(&expected, &root, Some(&root_identity))
            .expect("preview publication guard");
        for (path, destination) in [
            (&source, root.join("moved-source.png")),
            (&root, ancestor.join("moved-root")),
            (&ancestor, directory.path().join("moved-ancestor")),
        ] {
            let error = match fs::rename(path, destination) {
                Ok(()) => {
                    panic!("the live preview guard allowed namespace rebinding at {path:?}")
                }
                Err(error) => error,
            };
            assert!(matches!(error.raw_os_error(), Some(5) | Some(32)));
        }
        drop(guard);

        let moved_root = ancestor.join("released-root");
        rename_disposable_directory(&root, &moved_root, "root rename after preview guard drop");
        rename_disposable_directory(&moved_root, &root, "restore root after preview guard drop");
        let moved_ancestor = directory.path().join("released-ancestor");
        rename_disposable_directory(
            &ancestor,
            &moved_ancestor,
            "ancestor rename after preview guard drop",
        );
        rename_disposable_directory(
            &moved_ancestor,
            &ancestor,
            "restore ancestor after preview guard drop",
        );
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_cursor_retains_its_own_publication_guard() {
        let directory = tempdir_in(std::env::current_dir().expect("current directory"))
            .expect("ordinary-user disposable directory");
        let root = directory.path().join("root");
        let moved = directory.path().join("moved-root");
        let album = root.join("album");
        fs::create_dir_all(&album).expect("source album");
        fs::write(album.join("photo.jpg"), b"inventory fixture").expect("source entry");
        let expected_identity = FileDiscovery::new(&root.to_string_lossy())
            .expect("source discovery")
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");
        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &root.to_string_lossy(),
            &expected_identity,
        )
        .expect("publication guard");
        let mut entries = guard
            .streaming_metadata_inventory_entries_in_directory("album")
            .expect("guard-owned inventory cursor");
        assert!(entries.directory_identity().is_some());
        drop(guard);

        let held_rename = fs::rename(&root, &moved)
            .expect_err("the cursor-owned guard must retain the configured namespace");
        assert_eq!(held_rename.raw_os_error(), Some(32));
        assert_eq!(
            entries
                .next()
                .expect("one inventory entry")
                .expect("read inventory entry")
                .relative_path,
            "album/photo.jpg"
        );
        assert!(entries.next().is_none());
        entries.finish().expect("finish guarded enumeration");

        rename_disposable_directory(&root, &moved, "rename after cursor guard release");
        rename_disposable_directory(&moved, &root, "restore source root");
    }

    #[cfg(windows)]
    #[test]
    fn publication_guard_pins_configured_root_and_temp_owned_ancestors_until_drop() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("long-path temporary directory");
        let first = directory.path().join("first");
        let second = first.join("second");
        let root = second.join("root");
        fs::create_dir_all(&root).expect("nested source root");
        let expected_identity = FileDiscovery::new(&root.to_string_lossy())
            .expect("source discovery")
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");
        PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow_mut().clear());
        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &root.to_string_lossy(),
            &expected_identity,
        )
        .expect("full configured namespace guard");
        let guard_facts = PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow().clone());
        assert!(guard_facts.len() >= 3);
        assert!(guard_facts.iter().all(|fact| {
            fact.desired_access == (FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
                && fact.share_access == (FILE_SHARE_READ | FILE_SHARE_WRITE)
                && fact.flags == (FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                && fact.flags & FILE_FLAG_OPEN_NO_RECALL == 0
        }));

        for (path, moved) in [
            (&root, second.join("moved-root")),
            (&second, first.join("moved-second")),
            (&first, directory.path().join("moved-first")),
        ] {
            let error = fs::rename(path, moved).expect_err("held namespace rename must fail");
            assert!(matches!(error.raw_os_error(), Some(5) | Some(32)));
        }
        let delete_error = fs::remove_dir(&root).expect_err("held root delete must fail");
        assert!(matches!(delete_error.raw_os_error(), Some(5) | Some(32)));
        drop(guard);

        let moved_root = second.join("released-root");
        rename_disposable_directory(&root, &moved_root, "root rename after drop");
        rename_disposable_directory(&moved_root, &root, "restore root");
        let moved_second = first.join("released-second");
        rename_disposable_directory(&second, &moved_second, "parent rename after drop");
        rename_disposable_directory(&moved_second, &second, "restore parent");
        let moved_first = directory.path().join("released-first");
        rename_disposable_directory(&first, &moved_first, "ancestor rename after drop");
        rename_disposable_directory(&moved_first, &first, "restore ancestor");
        fs::remove_dir(&root).expect("root delete after guard drop");
    }

    #[cfg(windows)]
    #[test]
    fn metadata_inventory_source_guard_rejects_a_replaced_ancestor_before_open() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("long-path temporary directory");
        let ancestor = directory.path().join("configured-ancestor");
        let root = ancestor.join("library-root");
        fs::create_dir_all(&root).expect("nested source root");
        let expected_identity = FileDiscovery::new(&root.to_string_lossy())
            .expect("source discovery")
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");
        let retained_ancestor = directory.path().join("retained-configured-ancestor");
        rename_disposable_directory(&ancestor, &retained_ancestor, "retain authorized ancestor");
        fs::create_dir_all(&root).expect("replacement ancestor and root");

        let error = match PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
            &root.to_string_lossy(),
            Some(&expected_identity),
        ) {
            Ok(_) => panic!("a replacement ancestor namespace must not inherit source authority"),
            Err(error) => error,
        };
        assert_eq!(error.code, "metadata_inventory_root_identity_changed");

        fs::remove_dir_all(&ancestor).expect("remove replacement ancestor");
        rename_disposable_directory(&retained_ancestor, &ancestor, "restore authorized ancestor");
    }

    #[cfg(windows)]
    #[test]
    fn directory_guard_createfile_matrix_uses_long_path_and_no_delete_sharing() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("long-path temporary directory");
        let short_root = directory.path().join("matrix-root");
        let moved_root = directory.path().join("matrix-root-moved");
        fs::create_dir(&short_root).expect("matrix root");
        let long_root = windows_long_dos_path(&short_root).expect("long DOS path");
        eprintln!(
            "createfile_guard_matrix short={:?} long={:?}",
            short_root, long_root
        );
        let (volume_root, components) = publication_namespace_path(&long_root).expect("path parts");
        let mut prefix = volume_root;
        for component in components {
            prefix.push(component);
            for (access_name, desired_access) in [
                ("read-ea", FILE_READ_EA),
                ("read-control", READ_CONTROL),
                ("list", FILE_LIST_DIRECTORY),
                ("traverse", FILE_TRAVERSE),
            ] {
                for (share_name, share_mode) in [
                    ("no-delete", FILE_SHARE_READ | FILE_SHARE_WRITE),
                    (
                        "share-delete",
                        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    ),
                ] {
                    let opened = OpenOptions::new()
                        .access_mode(desired_access)
                        .share_mode(share_mode)
                        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                        .open(&prefix);
                    eprintln!(
                        "createfile_guard_matrix prefix={prefix:?} {access_name}-{share_name}={opened:?}"
                    );
                }
            }
        }

        let combinations = [
            ("short-reparse-no-delete", short_root.as_path(), true, false),
            ("long-follow-no-delete", long_root.as_path(), false, false),
            ("long-reparse-share-delete", long_root.as_path(), true, true),
            ("long-reparse-no-delete", long_root.as_path(), true, false),
        ];
        for (name, path, open_reparse, share_delete) in combinations {
            let flags = FILE_FLAG_BACKUP_SEMANTICS
                | if open_reparse {
                    FILE_FLAG_OPEN_REPARSE_POINT
                } else {
                    0
                };
            let opened = OpenOptions::new()
                .access_mode(0)
                .share_mode(
                    FILE_SHARE_READ
                        | FILE_SHARE_WRITE
                        | if share_delete { FILE_SHARE_DELETE } else { 0 },
                )
                .custom_flags(flags)
                .open(path);
            eprintln!("createfile_guard_matrix {name}={opened:?}");
            let handle = opened.unwrap_or_else(|error| panic!("{name} must open: {error}"));
            raw_file_id_info(&handle)
                .unwrap_or_else(|error| panic!("{name} handle must remain live: {error}"));
        }

        let access_combinations = [
            ("zero", 0),
            ("read-attributes", FILE_READ_ATTRIBUTES),
            ("synchronize", SYNCHRONIZE),
            (
                "read-attributes-synchronize",
                FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            ),
            ("read-ea", FILE_READ_EA),
            ("read-control", READ_CONTROL),
            ("list-directory", FILE_LIST_DIRECTORY),
            ("traverse", FILE_TRAVERSE),
        ];
        let mut blocking_access = None;
        for (name, desired_access) in access_combinations {
            let guard = OpenOptions::new()
                .access_mode(desired_access)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(&long_root)
                .unwrap_or_else(|error| panic!("{name} guard must open: {error}"));
            raw_file_id_info(&guard)
                .unwrap_or_else(|error| panic!("{name} guard must remain live: {error}"));
            let rename = fs::rename(&long_root, &moved_root);
            eprintln!("createfile_guard_matrix access={name} held_rename={rename:?}");
            match rename {
                Ok(()) => {
                    drop(guard);
                    fs::rename(&moved_root, &long_root).expect("restore renamed matrix root");
                }
                Err(error) => {
                    assert_eq!(error.raw_os_error(), Some(32));
                    blocking_access.get_or_insert(desired_access);
                    drop(guard);
                }
            }
        }
        let blocking_access = blocking_access.expect("one minimal access must pin rename");
        let guard = OpenOptions::new()
            .access_mode(blocking_access)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&long_root)
            .expect("blocking guard");
        let held_delete = fs::remove_dir(&long_root).expect_err("held delete must fail");
        eprintln!(
            "createfile_guard_matrix blocking_access={blocking_access:#x} held_delete={held_delete:?}"
        );
        assert_eq!(held_delete.raw_os_error(), Some(32));
        drop(guard);
        fs::remove_dir(&long_root).expect("delete after blocking guard drop");
    }

    #[cfg(windows)]
    #[test]
    fn ordinary_user_profile_boundary_supports_a_full_publication_guard() {
        let user_profile =
            PathBuf::from(std::env::var_os("USERPROFILE").expect("Windows user profile path"));
        let profile_parent = user_profile.parent().expect("user profile parent");
        validate_publication_namespace_path(&user_profile).expect("user profile path evidence");
        match open_publication_namespace_guard(&user_profile) {
            Ok(guard) => drop(guard),
            Err(error) if publication_namespace_access_is_denied(&error) => {
                require_publication_namespace_acl_boundary(&user_profile, profile_parent)
                    .expect("the inaccessible profile prefix must be ACL-protected");
            }
            Err(error) => panic!("unexpected user-profile guard failure: {error}"),
        }

        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("ordinary-user disposable directory");
        let namespace = directory.path().join("namespace");
        let source = namespace.join("source");
        fs::create_dir_all(&source).expect("ordinary-user disposable source root");
        assert!(
            require_publication_namespace_acl_boundary(&source, &namespace).is_err(),
            "a caller-mutable namespace must never be accepted as an ACL trust boundary"
        );
        let expected_identity = FileDiscovery::new(&source.to_string_lossy())
            .expect("source discovery")
            .metadata_inventory_root_identity()
            .expect("root identity query")
            .expect("stable root identity");
        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &source.to_string_lossy(),
            &expected_identity,
        )
        .expect("full ordinary-user namespace guard");
        let moved_source = namespace.join("moved-source");
        let held_rename = fs::rename(&source, &moved_source)
            .expect_err("full ordinary-user namespace guard must pin the configured root");
        assert!(matches!(held_rename.raw_os_error(), Some(5) | Some(32)));
        drop(guard);
        rename_disposable_directory(
            &source,
            &moved_source,
            "rename after ordinary-user namespace guard release",
        );
        rename_disposable_directory(
            &moved_source,
            &source,
            "restore ordinary-user disposable source root",
        );
    }

    #[cfg(windows)]
    #[test]
    fn win32_traverse_guard_helper_pins_each_temp_owned_namespace_layer() {
        let current_directory = std::env::current_dir().expect("current directory");
        let directory = tempdir_in(current_directory).expect("long-path temporary directory");
        let first = directory.path().join("first");
        let second = first.join("second");
        let root = second.join("root");
        fs::create_dir_all(&root).expect("nested publication namespace");
        let layers = [first.clone(), second.clone(), root.clone()];
        PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow_mut().clear());
        let guards = layers
            .iter()
            .map(|layer| {
                let guard = open_publication_namespace_guard(layer)
                    .unwrap_or_else(|error| panic!("guard {layer:?}: {error}"));
                raw_file_id_info(&guard).expect("live production guard handle");
                guard
            })
            .collect::<Vec<_>>();
        let facts = PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow().clone());
        assert_eq!(facts.len(), layers.len());
        for (fact, layer) in facts.iter().zip(layers.iter()) {
            assert_eq!(&fact.path, layer);
            assert_eq!(
                fact.desired_access,
                FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE
            );
            assert_eq!(fact.share_access, FILE_SHARE_READ | FILE_SHARE_WRITE);
            assert_eq!(
                fact.flags,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT
            );
            assert_eq!(fact.flags & FILE_FLAG_OPEN_NO_RECALL, 0);
        }
        for (index, layer) in layers.iter().enumerate() {
            let moved = layer.with_file_name(format!("held-layer-{index}"));
            let error = fs::rename(layer, &moved).expect_err("held layer rename must fail");
            assert!(matches!(error.raw_os_error(), Some(5) | Some(32)));
        }
        let delete_error = fs::remove_dir(&root).expect_err("held root delete must fail");
        assert!(matches!(delete_error.raw_os_error(), Some(5) | Some(32)));
        drop(guards);
        for (index, layer) in layers.iter().enumerate().rev() {
            let moved = layer.with_file_name(format!("released-layer-{index}"));
            rename_disposable_directory(layer, &moved, "rename after all guards drop");
            rename_disposable_directory(&moved, layer, "restore released layer");
        }
        fs::remove_dir(&root).expect("delete root after all guards drop");
    }

    #[cfg(windows)]
    #[test]
    fn raw_volume_roots_are_rejected_before_any_handle_open() {
        CONFIGURED_ROOT_OPENS.with(|facts| facts.borrow_mut().clear());
        PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow_mut().clear());
        LAST_ROOT_RELATIVE_METADATA_OPEN.with(|facts| facts.set(None));
        LAST_ROOT_RELATIVE_DATA_OPEN.with(|facts| facts.set(None));

        let error = match FileDiscovery::new(r"\\.\C:") {
            Ok(_) => panic!("raw volume root must fail closed"),
            Err(error) => error,
        };

        assert_eq!(error.code, "root_device_namespace_unsupported");
        assert!(CONFIGURED_ROOT_OPENS.with(|facts| facts.borrow().is_empty()));
        assert!(PUBLICATION_NAMESPACE_GUARD_OPENS.with(|facts| facts.borrow().is_empty()));
        assert_eq!(LAST_ROOT_RELATIVE_METADATA_OPEN.with(Cell::get), None);
        assert_eq!(LAST_ROOT_RELATIVE_DATA_OPEN.with(Cell::get), None);
    }

    #[cfg(windows)]
    #[test]
    fn ordinal_component_contract_distinguishes_sensitive_names() {
        assert!(
            windows_ordinal_equals("Photos".as_ref(), "photos".as_ref(), true)
                .expect("case-insensitive comparison")
        );
        assert!(
            !windows_ordinal_equals("Photos".as_ref(), "photos".as_ref(), false)
                .expect("case-sensitive comparison")
        );
    }

    #[cfg(windows)]
    #[test]
    fn offline_evidence_attempts_zero_content_access_upgrades() {
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_OFFLINE;

        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("offline.png");
        fs::write(&path, b"must-not-be-read").expect("source fixture");
        let metadata_handle = open_validation_handle(&path).expect("metadata handle");
        let (root_proof, _) =
            WindowsDirectoryRootProof::open_configured(directory.path(), true).expect("root proof");
        PINNED_SOURCE_ACCESS_UPGRADE_COUNT.with(|count| count.set(0));
        PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(|count| count.set(0));
        LAST_ROOT_RELATIVE_DATA_OPEN.with(|facts| facts.set(None));

        let error = open_validated_root_relative_source_file(
            directory.path(),
            &root_proof,
            Path::new("offline.png"),
            &metadata_handle,
            AttributeTagEvidence {
                attributes: FILE_ATTRIBUTE_OFFLINE,
                reparse_tag: 0,
            },
        )
        .expect_err("offline evidence must fail before content access");

        assert_eq!(PINNED_SOURCE_ACCESS_UPGRADE_COUNT.with(Cell::get), 0);
        assert_eq!(PINNED_SOURCE_DATA_AUTHORIZATION_COUNT.with(Cell::get), 0);
        assert_eq!(LAST_ROOT_RELATIVE_DATA_OPEN.with(Cell::get), None);
        assert!(error.to_string().contains("not locally available"));
    }

    #[cfg(windows)]
    #[test]
    fn final_present_revalidation_uses_one_no_recall_attribute_handle() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("local.png");
        fs::write(&path, b"local-content").expect("source fixture");
        let metadata = fs::metadata(&path).expect("source metadata");
        let (file_identity, source_revision) =
            file_source_evidence(&path).expect("source evidence");
        let expected = ExpectedFileState {
            absolute_path: path_text(&path),
            file_size: metadata.len(),
            modified_unix_ms: modified_unix_ms(&metadata),
            file_identity,
            source_revision: Some(source_revision),
        };
        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");
        LAST_ROOT_RELATIVE_DATA_OPEN.with(|facts| facts.set(None));

        discovery
            .revalidate_relative_file_state("local.png", &expected)
            .expect("present local source");

        let facts = LAST_ROOT_RELATIVE_DATA_OPEN
            .with(Cell::get)
            .expect("root-relative revalidation open facts");
        assert_eq!(facts.desired_access, FILE_READ_ATTRIBUTES | SYNCHRONIZE);
        assert_eq!(
            facts.share_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
        );
        assert_ne!(facts.create_options & FILE_OPEN_REPARSE_POINT, 0);
        assert_ne!(facts.create_options & FILE_OPEN_NO_RECALL_NATIVE, 0);
        assert_ne!(facts.create_options & FILE_NON_DIRECTORY_FILE, 0);
        assert!(facts.ea_buffer_is_null);
        assert_eq!(facts.ea_length, 0);
        assert!(!facts.object_name_is_absolute);
        assert!(!facts.object_name_has_separator);
    }

    #[cfg(windows)]
    #[test]
    fn terminal_root_relative_data_open_carries_native_no_recall() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("local.png");
        fs::write(&path, b"local-content").expect("source fixture");
        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");
        LAST_ROOT_RELATIVE_METADATA_OPEN.with(|facts| facts.set(None));
        LAST_ROOT_RELATIVE_DATA_OPEN.with(|facts| facts.set(None));

        let mut reopened = discovery
            .open_pinned_source_file("local.png")
            .expect("terminal no-recall data open");
        let mut bytes = Vec::new();
        reopened.read_to_end(&mut bytes).expect("read local source");

        assert_eq!(bytes, b"local-content");
        let metadata_facts = LAST_ROOT_RELATIVE_METADATA_OPEN
            .with(Cell::get)
            .expect("metadata open facts");
        assert_eq!(
            metadata_facts.desired_access,
            FILE_READ_ATTRIBUTES | SYNCHRONIZE
        );
        assert_eq!(
            metadata_facts.share_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
        );
        assert_eq!(
            metadata_facts.create_options & FILE_OPEN_NO_RECALL_NATIVE,
            0
        );
        assert!(metadata_facts.ea_buffer_is_null);
        assert_eq!(metadata_facts.ea_length, 0);
        assert!(!metadata_facts.object_name_is_absolute);
        assert!(!metadata_facts.object_name_has_separator);
        let data_facts = LAST_ROOT_RELATIVE_DATA_OPEN
            .with(Cell::get)
            .expect("data open facts");
        assert_eq!(
            data_facts.desired_access,
            FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        );
        assert_eq!(data_facts.share_access, FILE_SHARE_READ | FILE_SHARE_WRITE);
        assert_ne!(data_facts.create_options & FILE_OPEN_NO_RECALL_NATIVE, 0);
        assert_ne!(data_facts.create_options & FILE_OPEN_REPARSE_POINT, 0);
        assert!(data_facts.ea_buffer_is_null);
        assert_eq!(data_facts.ea_length, 0);
        assert!(!data_facts.object_name_is_absolute);
        assert!(!data_facts.object_name_has_separator);
    }

    #[cfg(windows)]
    #[test]
    fn handle_directory_buffer_parser_preserves_metadata_and_rejects_malformed_offsets() {
        fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
            buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }

        fn write_i64(buffer: &mut [u8], offset: usize, value: i64) {
            buffer[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }

        fn write_record(
            buffer: &mut [u8],
            next_offset: u32,
            name: &str,
            attributes: u32,
            file_id: u128,
        ) {
            let name = name.encode_utf16().collect::<Vec<_>>();
            write_u32(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
                next_offset,
            );
            write_i64(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, LastWriteTime),
                WINDOWS_TO_UNIX_EPOCH_100NS + 42 * HUNDRED_NS_PER_MILLISECOND,
            );
            write_i64(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ChangeTime),
                0x0123_4567_89ab_cdef,
            );
            write_i64(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, EndOfFile),
                7,
            );
            write_u32(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileAttributes),
                attributes,
            );
            write_u32(
                buffer,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileNameLength),
                u32::try_from(name.len() * size_of::<u16>()).expect("name length"),
            );
            let id_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileId);
            buffer[id_offset..id_offset + 16].copy_from_slice(&file_id.to_le_bytes());
            let name_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
            for (index, unit) in name.into_iter().enumerate() {
                let offset = name_offset + index * size_of::<u16>();
                buffer[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
            }
        }

        let mut buffer = vec![0_u8; 512];
        write_record(&mut buffer[..256], 256, "first.png", 0, 11);
        write_record(
            &mut buffer[256..],
            0,
            "nested",
            FILE_ATTRIBUTE_DIRECTORY,
            12,
        );

        let entries = parse_handle_directory_buffer(&buffer, 9, "album").expect("valid records");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].relative_path, "album/first.png");
        assert_eq!(entries[0].kind, MetadataInventoryEntryKind::File);
        assert_eq!(entries[0].file_size, Some(7));
        assert_eq!(entries[0].modified_unix_ms, 42);
        assert_eq!(
            entries[0].source_revision,
            Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0123456789abcdef".to_owned(),
            })
        );
        assert_eq!(
            entries[0]
                .file_identity
                .as_ref()
                .map(|identity| identity.value.as_str()),
            Some("0000000000000009:0000000000000000000000000000000b")
        );
        assert_eq!(entries[1].relative_path, "album/nested");
        assert_eq!(entries[1].kind, MetadataInventoryEntryKind::Directory);
        assert!(entries[1].file_identity.is_none());

        write_u32(
            &mut buffer,
            std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
            3,
        );
        assert!(parse_handle_directory_buffer(&buffer, 9, "album").is_err());
    }

    #[test]
    fn reparse_cloud_placeholder_remains_a_present_file_entry() {
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::CloudFiles),
            MetadataInventoryEntryKind::File
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, false, ReparseKind::CloudFiles),
            MetadataInventoryEntryKind::Other
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::Other),
            MetadataInventoryEntryKind::Other
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::None),
            MetadataInventoryEntryKind::File
        );
    }

    #[cfg(windows)]
    #[test]
    fn cloud_files_tag_distinguishes_hydrated_placeholder_from_other_reparse_points() {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_OPEN,
        };
        use windows_sys::Win32::System::SystemServices::{
            IO_REPARSE_TAG_CLOUD_2, IO_REPARSE_TAG_SYMLINK,
        };

        let cloud = reparse_evidence_from_attribute_tag(
            FILE_ATTRIBUTE_REPARSE_POINT,
            IO_REPARSE_TAG_CLOUD_2,
        )
        .expect("Cloud Files tag state");
        let handle_info = FILE_ATTRIBUTE_TAG_INFO {
            FileAttributes: FILE_ATTRIBUTE_REPARSE_POINT,
            ReparseTag: IO_REPARSE_TAG_CLOUD_2,
        };
        let enumeration_info = WIN32_FIND_DATAW {
            dwFileAttributes: FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_RECALL_ON_OPEN,
            dwReserved0: IO_REPARSE_TAG_CLOUD_2,
            ..WIN32_FIND_DATAW::default()
        };
        let exact_evidence = combine_attribute_tag_evidence(0, &handle_info, &enumeration_info)
            .expect("exact enumeration evidence");
        let recall = reparse_evidence_from_attribute_tag(
            exact_evidence.attributes,
            exact_evidence.reparse_tag,
        )
        .expect("recall Cloud Files tag state");
        let symlink = reparse_evidence_from_attribute_tag(
            FILE_ATTRIBUTE_REPARSE_POINT,
            IO_REPARSE_TAG_SYMLINK,
        )
        .expect("symlink tag state");

        assert_eq!(
            cloud,
            (
                ReparseKind::CloudFiles,
                MetadataInventoryPlaceholderState::Available,
            )
        );
        assert_eq!(
            recall,
            (
                ReparseKind::CloudFiles,
                MetadataInventoryPlaceholderState::RecallOnOpen,
            )
        );
        assert_eq!(
            symlink,
            (
                ReparseKind::Other,
                MetadataInventoryPlaceholderState::Available,
            )
        );

        let fixture_path = Path::new(r"C:\library\photo.png");
        validate_present_file_revalidation_attributes(fixture_path, &handle_info)
            .expect("available Cloud Files entry remains present");
        let recall_issue = validate_present_file_revalidation_attributes(
            fixture_path,
            &FILE_ATTRIBUTE_TAG_INFO {
                FileAttributes: FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_RECALL_ON_OPEN,
                ReparseTag: IO_REPARSE_TAG_CLOUD_2,
            },
        )
        .expect_err("recall-on-open Cloud Files entry is not present locally");
        assert_eq!(recall_issue.code, "cloud_placeholder_skipped");
        let offline_issue = validate_present_file_revalidation_attributes(
            fixture_path,
            &FILE_ATTRIBUTE_TAG_INFO {
                FileAttributes: FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_OFFLINE,
                ReparseTag: IO_REPARSE_TAG_CLOUD_2,
            },
        )
        .expect_err("offline Cloud Files entry is not present locally");
        assert_eq!(offline_issue.code, "cloud_placeholder_skipped");
        let symlink_issue = validate_present_file_revalidation_attributes(
            fixture_path,
            &FILE_ATTRIBUTE_TAG_INFO {
                FileAttributes: FILE_ATTRIBUTE_REPARSE_POINT,
                ReparseTag: IO_REPARSE_TAG_SYMLINK,
            },
        )
        .expect_err("non-Cloud reparse entry is not a present file");
        assert_eq!(symlink_issue.code, "source_path_outside_root");
    }

    #[cfg(windows)]
    #[test]
    fn exact_enumeration_uses_an_extended_literal_path() {
        let path = Path::new(r"C:\library\图片\photo.png");
        let wide = windows_extended_path(path);
        let rendered = String::from_utf16(&wide[..wide.len() - 1]).expect("UTF-16 path");

        assert_eq!(rendered, r"\\?\C:\library\图片\photo.png");
        assert_eq!(wide.last(), Some(&0));
    }

    #[cfg(windows)]
    #[test]
    fn windows_identity_survives_rename_and_distinguishes_a_replacement() {
        let directory = tempdir().expect("temporary directory");
        let original_path = directory.path().join("original.bin");
        let moved_path = directory.path().join("moved.bin");
        fs::write(&original_path, b"same-size").expect("original fixture");
        let original_identity = file_identity(&original_path)
            .expect("original identity")
            .expect("Windows identity");

        fs::rename(&original_path, &moved_path).expect("rename fixture");
        let moved_identity = file_identity(&moved_path)
            .expect("moved identity")
            .expect("Windows identity");
        fs::write(&original_path, b"same-size").expect("replacement fixture");
        let replacement_identity = file_identity(&original_path)
            .expect("replacement identity")
            .expect("Windows identity");

        assert_eq!(moved_identity, original_identity);
        assert_ne!(replacement_identity, original_identity);

        let replacement_metadata = fs::metadata(&original_path).expect("replacement metadata");
        let error = revalidate_file_state(&ExpectedFileState {
            absolute_path: path_text(&original_path),
            file_size: replacement_metadata.len(),
            modified_unix_ms: modified_unix_ms(&replacement_metadata),
            file_identity: Some(original_identity),
            source_revision: None,
        })
        .expect_err("replacement identity must be rejected");
        assert_eq!(error.code, "source_replaced_during_scan");
    }

    #[cfg(windows)]
    #[test]
    fn windows_revision_rejects_same_size_in_place_edit_with_restored_mtime() {
        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.bin");
        fs::write(&source_path, b"original").expect("original fixture");
        let original_metadata = fs::metadata(&source_path).expect("original metadata");
        let original_modified = original_metadata
            .modified()
            .expect("original modified time");
        let (original_identity, original_revision) =
            file_source_evidence(&source_path).expect("original source evidence");
        let expected = ExpectedFileState {
            absolute_path: path_text(&source_path),
            file_size: original_metadata.len(),
            modified_unix_ms: modified_unix_ms(&original_metadata),
            file_identity: original_identity.clone(),
            source_revision: Some(original_revision.clone()),
        };

        thread::sleep(Duration::from_millis(2));
        fs::write(&source_path, b"changed!").expect("same-size in-place edit");
        let source = fs::OpenOptions::new()
            .write(true)
            .open(&source_path)
            .expect("open edited fixture");
        source
            .set_times(std::fs::FileTimes::new().set_modified(original_modified))
            .expect("restore modified time");
        drop(source);

        let edited_metadata = fs::metadata(&source_path).expect("edited metadata");
        let (edited_identity, edited_revision) =
            file_source_evidence(&source_path).expect("edited source evidence");
        assert_eq!(edited_metadata.len(), expected.file_size);
        assert_eq!(
            modified_unix_ms(&edited_metadata),
            expected.modified_unix_ms
        );
        assert_eq!(edited_identity, original_identity);
        assert_ne!(edited_revision, original_revision);
        let error = revalidate_file_state(&expected)
            .expect_err("restored mtime must not hide an in-place content edit");
        assert_eq!(error.code, "source_revision_changed_during_scan");
    }

    #[test]
    fn root_availability_distinguishes_available_and_missing_paths() {
        let directory = tempdir().expect("temporary directory");
        let root_path = directory.path().to_string_lossy();
        let missing_path = directory.path().join("missing");
        let missing_path_text = missing_path.to_string_lossy();
        reset_root_availability_metadata_probe_instrumentation(&root_path);
        reset_root_availability_metadata_probe_instrumentation(&missing_path_text);
        let available = inspect_root_availability(&root_path);
        let missing = inspect_root_availability(&missing_path_text);

        assert!(matches!(
            available.availability,
            LibraryRootAvailability::Available
        ));
        assert!(available.message.is_none());
        assert!(matches!(
            missing.availability,
            LibraryRootAvailability::Missing
        ));
        assert!(missing.message.is_some());
        assert_eq!(root_availability_metadata_probe_count(&root_path), 1);
        assert_eq!(
            root_availability_metadata_probe_count(&missing_path_text),
            1
        );
    }

    #[test]
    fn root_availability_algorithm_has_a_metadata_only_constant_work_contract() {
        let evidence = classify_root_availability_metadata(
            RootAvailabilityMetadataEvidence::AvailableDirectory,
        );
        assert!(matches!(
            evidence.availability,
            LibraryRootAvailability::Available
        ));
    }

    #[test]
    fn root_availability_owning_functions_reject_enumeration_source() {
        let source = include_str!("local_files.rs");
        let domain_source = include_str!("../domain/mod.rs");
        let metadata_domain_source = include_str!("../domain/library_metadata_inventory.rs");
        let adapters_parent_source = include_str!("mod.rs");
        let crate_parent_source = include_str!("../lib.rs");
        assert_availability_exact_source_contract_with_environment(
            source,
            Some(domain_source),
            Some(metadata_domain_source),
            Some(adapters_parent_source),
            Some(crate_parent_source),
        )
        .expect("current production availability AST must match its exact call closure");
        assert_no_relevant_availability_impls_in_crate()
            .expect("referenced availability types must not gain side-effecting impls");

        let representative_violation = r####"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _ordinary = "}";
    let _raw = r###"} read_dir( is inert"###;
    // } closes nothing
    let _entries = std::fs::read_dir(root_path);
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"####;
        let error = assert_availability_exact_source_contract(representative_violation, None, None)
            .expect_err("representative directory enumeration must make the guard red");
        assert!(error.contains("fn:inspect_root_availability"));

        let representative_legal = source.replacen(
            "pub fn inspect_root_availability(root_path: &str)",
            concat!(
                "// std::fs::read_dir(root_path) is inert review text.\n",
                "pub fn inspect_root_availability(root_path: &str)"
            ),
            1,
        );
        assert_availability_exact_source_contract(
            &representative_legal,
            Some(domain_source),
            Some(metadata_domain_source),
        )
        .expect("comments and whitespace must not affect the token contract");

        let broadened_file_system_import = source.replacen(
            "GetVolumeInformationByHandleW, SYNCHRONIZE, WIN32_FIND_DATAW, WRITE_DAC, WRITE_OWNER,",
            "GetVolumeInformationByHandleW, ReadDirectoryChangesW, SYNCHRONIZE, WIN32_FIND_DATAW, WRITE_DAC, WRITE_OWNER,",
            1,
        );
        assert_ne!(broadened_file_system_import, source);
        let error = assert_availability_exact_source_contract(
            &broadened_file_system_import,
            Some(domain_source),
            Some(metadata_domain_source),
        )
        .expect_err("an additional Win32 import must require another intentional signature");
        assert!(error.contains("local-use:file-system-api"));

        let broadened_domain_import = source.replacen(
            "SourceRevisionEvidence,",
            "SourceRevisionEvidence, UserOverride,",
            1,
        );
        assert_ne!(broadened_domain_import, source);
        let error = assert_availability_exact_source_contract(
            &broadened_domain_import,
            Some(domain_source),
            Some(metadata_domain_source),
        )
        .expect_err("an additional domain import must require another intentional signature");
        assert!(error.contains("local-use:domain-availability-types"));

        let moved_source_revision_docs = domain_source.replacen(
            "pub struct SourceRevisionEvidence",
            "pub struct RenamedSourceRevisionEvidence",
            1,
        );
        assert_ne!(moved_source_revision_docs, domain_source);
        let error = assert_availability_exact_source_contract(
            source,
            Some(&moved_source_revision_docs),
            Some(metadata_domain_source),
        )
        .expect_err("the exact SourceRevision documentation must not authorize another item");
        assert!(error.contains("unapproved top-level attribute doc"));

        let structural_violations = [
            (
                "module alias",
                r#"
use std::fs::read_dir as rd;
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _entries = rd(root_path);
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "local alias",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    use std::fs::read_dir as rd;
    let _entries = rd(root_path);
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "macro token",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _entries = dbg!(std::fs::read_dir(root_path));
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "external helper",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _entries = hidden_helper(root_path);
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "method alias",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let path = RootPath::new(root_path);
    let _entries = path.rd();
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "fully qualified direct call",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _entries = std::fs::read_dir(root_path);
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "closure",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _deferred = || root_path;
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "async block",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _deferred = async { root_path };
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
            (
                "unsafe block",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _value = unsafe { 1 };
    classify_root_availability_metadata(RootAvailabilityMetadataEvidence::AvailableDirectory)
}
"#,
            ),
        ];
        for (label, violation) in structural_violations {
            let result = assert_availability_exact_source_contract(violation, None, None);
            assert!(
                result.is_err(),
                "{label} must make the structural allowlist red"
            );
        }

        let sixth_review_violations = [
            (
                "callee local shadow",
                r#"
fn hidden_helper(
    evidence: RootAvailabilityMetadataEvidence,
) -> RootAvailabilityEvidence {
    let _entries = std::fs::read_dir(".");
    classify_root_availability_metadata(evidence)
}

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let classify_root_availability_metadata = hidden_helper;
    classify_root_availability_metadata(
        RootAvailabilityMetadataEvidence::AvailableDirectory,
    )
}
"#,
            ),
            (
                "receiver shadow",
                r#"
pub fn probe_root_availability_metadata(
    root_path: &str,
) -> RootAvailabilityMetadataEvidence {
    let path = malicious_receiver;
    path.symlink_metadata()
}
"#,
            ),
            (
                "unchanged owner with enumerating helper",
                r#"
pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    classify_root_availability_metadata(probe_root_availability_metadata(root_path))
}

fn probe_root_availability_metadata(
    root_path: &str,
) -> RootAvailabilityMetadataEvidence {
    let _entries = std::fs::read_dir(root_path);
    RootAvailabilityMetadataEvidence::AvailableDirectory
}
"#,
            ),
            (
                "Drop side effect",
                r#"
impl Drop for RootAvailabilityMetadataEvidence {
    fn drop(&mut self) {
        let _entries = std::fs::read_dir(".");
    }
}

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _bomb = RootAvailabilityMetadataEvidence::AvailableDirectory;
    classify_root_availability_metadata(
        RootAvailabilityMetadataEvidence::AvailableDirectory,
    )
}
"#,
            ),
            (
                "operator overload",
                r#"
impl std::ops::BitOr for RootAvailabilityMetadataEvidence {
    type Output = RootAvailabilityMetadataEvidence;

    fn bitor(self, _other: Self) -> Self::Output {
        let _entries = std::fs::read_dir(".");
        RootAvailabilityMetadataEvidence::AvailableDirectory
    }
}

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _probe = RootAvailabilityMetadataEvidence::AvailableDirectory
        | RootAvailabilityMetadataEvidence::AvailableDirectory;
    classify_root_availability_metadata(
        RootAvailabilityMetadataEvidence::AvailableDirectory,
    )
}
"#,
            ),
            (
                "static LazyLock side effect",
                r#"
static SIDE_EFFECT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    let _entries = std::fs::read_dir(".");
});

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let _side_effect = &*SIDE_EFFECT;
    classify_root_availability_metadata(
        RootAvailabilityMetadataEvidence::AvailableDirectory,
    )
}
"#,
            ),
        ];
        for (label, violation) in sixth_review_violations {
            let error = assert_availability_exact_source_contract(violation, None, None)
                .expect_err("sixth-review source must violate the exact contract");
            assert!(
                error.contains("availability contract item")
                    || error.contains("availability owner"),
                "{label} must report an owner-specific item key: {error}"
            );
        }
    }

    #[test]
    fn root_availability_contract_rejects_macro_resolution_environment_changes() {
        let source = include_str!("local_files.rs");
        let domain_source = include_str!("../domain/mod.rs");
        let metadata_domain_source = include_str!("../domain/library_metadata_inventory.rs");
        let adapters_parent_source = include_str!("mod.rs");
        let crate_parent_source = include_str!("../lib.rs");
        let macro_using_source = source.replacen(
            concat!(
                "        let mut extended = std::vec::Vec::with_capacity(4);\n",
                "        extended.extend_from_slice(&[separator, separator, u16::from(b'?'), separator]);"
            ),
            "        let mut extended = vec![separator, separator, u16::from(b'?'), separator];",
            1,
        );
        assert_ne!(
            macro_using_source, source,
            "the macro-resolution fixture must replace the protected constructor"
        );

        let same_module = format!(
            "macro_rules! vec {{ ($($token:tt)*) => {{ std::fs::read_dir(\".\").unwrap(); Vec::new() }}; }}\n{macro_using_source}"
        );
        let same_module_error = assert_availability_exact_source_contract_with_environment(
            &same_module,
            Some(domain_source),
            Some(metadata_domain_source),
            Some(adapters_parent_source),
            Some(crate_parent_source),
        )
        .expect_err("a same-module vec macro must make the availability contract red");
        assert!(
            same_module_error.contains("availability macro environment source local"),
            "same-module macro failure must identify its source: {same_module_error}"
        );

        let parent_with_macro = format!(
            "macro_rules! vec {{ ($($token:tt)*) => {{ std::fs::read_dir(\".\").unwrap(); Vec::new() }}; }}\n{adapters_parent_source}"
        );
        let parent_error = assert_availability_exact_source_contract_with_environment(
            &macro_using_source,
            Some(domain_source),
            Some(metadata_domain_source),
            Some(&parent_with_macro),
            Some(crate_parent_source),
        )
        .expect_err("a parent-module vec macro must make the availability contract red");
        assert!(
            parent_error.contains("availability macro environment source adapters-parent"),
            "parent macro failure must identify its source: {parent_error}"
        );

        let renamed_import = format!("use crate::adversarial_vec as vec;\n{macro_using_source}");
        let import_error = assert_availability_exact_source_contract_with_environment(
            &renamed_import,
            Some(domain_source),
            Some(metadata_domain_source),
            Some(adapters_parent_source),
            Some(crate_parent_source),
        )
        .expect_err("a renamed vec macro import must make the availability contract red");
        assert!(
            import_error.contains("availability macro environment source local"),
            "renamed macro import failure must identify its source: {import_error}"
        );
    }

    #[test]
    fn root_availability_contract_rejects_module_loading_environment_changes() {
        let source = include_str!("local_files.rs");
        let domain_source = include_str!("../domain/mod.rs");
        let metadata_domain_source = include_str!("../domain/library_metadata_inventory.rs");
        let adapters_parent_source = include_str!("mod.rs");
        let crate_parent_source = include_str!("../lib.rs");
        let adapters_parent_violations = [
            (
                "cfg_attr path",
                adapters_parent_source.replacen(
                    "mod local_files;",
                    "#[cfg_attr(not(test), path = \"alternate.rs\")]\nmod local_files;",
                    1,
                ),
            ),
            (
                "direct path",
                adapters_parent_source.replacen(
                    "mod local_files;",
                    "#[path = \"alternate.rs\"]\nmod local_files;",
                    1,
                ),
            ),
            (
                "include item macro",
                format!("include!(\"alternate.rs\");\n{adapters_parent_source}"),
            ),
            (
                "extern crate alias",
                format!("extern crate alternate as local_files;\n{adapters_parent_source}"),
            ),
            (
                "nested generated module",
                format!(
                    "mod generated {{ include!(\"alternate.rs\"); }}\n{adapters_parent_source}"
                ),
            ),
            (
                "procedure-style module attribute",
                adapters_parent_source.replacen(
                    "mod local_files;",
                    "#[adversarial_loader]\nmod local_files;",
                    1,
                ),
            ),
        ];
        for (label, adapters_parent_violation) in adapters_parent_violations {
            assert_ne!(
                adapters_parent_violation, adapters_parent_source,
                "{label} fixture must change the adapters module source"
            );
            let error = assert_availability_exact_source_contract_with_environment(
                source,
                Some(domain_source),
                Some(metadata_domain_source),
                Some(&adapters_parent_violation),
                Some(crate_parent_source),
            )
            .expect_err("module loading mutation must make the availability contract red");
            assert!(
                error.contains("availability module topology source adapters-parent"),
                "{label} must identify the protected module topology source: {error}"
            );
        }

        let crate_parent_violation = crate_parent_source.replacen(
            "mod adapters;",
            "#[cfg_attr(not(test), path = \"alternate_adapters.rs\")]\nmod adapters;",
            1,
        );
        let crate_error = assert_availability_exact_source_contract_with_environment(
            source,
            Some(domain_source),
            Some(metadata_domain_source),
            Some(adapters_parent_source),
            Some(&crate_parent_violation),
        )
        .expect_err("crate-to-adapters loading mutation must make the availability contract red");
        assert!(
            crate_error.contains("availability module topology source crate-parent"),
            "crate loading mutation must identify the crate module topology: {crate_error}"
        );

        let domain_violation = domain_source.replacen(
            "mod library_metadata_inventory;",
            "#[path = \"alternate_metadata.rs\"]\nmod library_metadata_inventory;",
            1,
        );
        let domain_error = assert_availability_exact_source_contract_with_environment(
            source,
            Some(&domain_violation),
            Some(metadata_domain_source),
            Some(adapters_parent_source),
            Some(crate_parent_source),
        )
        .expect_err("domain metadata loading mutation must make the availability contract red");
        assert!(
            domain_error.contains("availability module topology source domain"),
            "domain loading mutation must identify the domain module topology: {domain_error}"
        );
    }

    #[derive(Clone, Copy)]
    struct AvailabilityFunctionContract {
        key: &'static str,
        digest: &'static str,
        local_callees: &'static [&'static str],
        summary: &'static str,
    }

    #[derive(Clone, Copy)]
    struct AvailabilityModuleContract {
        name: &'static str,
        visibility: &'static str,
        attributes: &'static [&'static str],
        is_inline: bool,
    }

    const CRATE_PARENT_MODULE_CONTRACTS: &[AvailabilityModuleContract] = &[
        AvailabilityModuleContract {
            name: "adapters",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "api",
            visibility: "pub",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "application",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "domain",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "frb_generated",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "journal_broker",
            visibility: "pub",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "media_fixtures",
            visibility: "pub(crate)",
            attributes: &["cfg(test)", "path=\"../test_support/media_fixtures.rs\""],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "ports",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "synchronization",
            visibility: "pub",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "windows_usn",
            visibility: "",
            attributes: &["cfg(windows)"],
            is_inline: false,
        },
    ];

    const ADAPTERS_PARENT_MODULE_CONTRACTS: &[AvailabilityModuleContract] = &[
        AvailabilityModuleContract {
            name: "durable_local_metadata_inventory",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "exif_metadata",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "image_orientation",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "jpeg_preview",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "local_files",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "local_metadata_inventory",
            visibility: "",
            attributes: &["cfg(test)"],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "media_inspector",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "preview_cache",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "sqlite_catalog",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "storage_settings",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "windows_library_change_source",
            visibility: "",
            attributes: &["cfg(windows)"],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "windows_usn_catch_up",
            visibility: "",
            attributes: &["cfg(all(windows,test))"],
            is_inline: false,
        },
    ];

    const DOMAIN_MODULE_CONTRACTS: &[AvailabilityModuleContract] = &[
        AvailabilityModuleContract {
            name: "gallery_query_snapshot",
            visibility: "pub",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_catalog_delta",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_change",
            visibility: "pub(crate)",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_change_catch_up",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_change_queue",
            visibility: "pub(crate)",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_metadata_inventory",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "library_synchronization",
            visibility: "pub(crate)",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "persistent_journal",
            visibility: "pub(crate)",
            attributes: &[],
            is_inline: false,
        },
    ];

    const LOCAL_MODULE_CONTRACTS: &[AvailabilityModuleContract] = &[
        AvailabilityModuleContract {
            name: "catalog_identity",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "preview_cache_namespace",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "file_admission",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "media_signature",
            visibility: "",
            attributes: &[],
            is_inline: false,
        },
        AvailabilityModuleContract {
            name: "tests",
            visibility: "",
            attributes: &["cfg(test)"],
            is_inline: true,
        },
        AvailabilityModuleContract {
            name: "viewer_source_guard",
            visibility: "",
            attributes: &["cfg(windows)"],
            is_inline: false,
        },
    ];

    const AVAILABILITY_FUNCTION_CONTRACTS: &[AvailabilityFunctionContract] = &[
        AvailabilityFunctionContract {
            key: "fn:inspect_root_availability",
            digest: "8ccdc0401ca16d22e10c304b5ac1e95a665797611988d23b5d9be97c5f88e3eb",
            local_callees: &[
                "classify_root_availability_metadata",
                "probe_root_availability_metadata",
            ],
            summary: "O(1) public availability entry",
        },
        AvailabilityFunctionContract {
            key: "fn:probe_root_availability_metadata",
            digest: "31777159d4d1acf416e2ab253c14891bc27f4f7f23252a494b8c7affeda41cce",
            local_callees: &["entry_reparse_evidence"],
            summary: "one no-follow metadata probe and error classification",
        },
        AvailabilityFunctionContract {
            key: "fn:classify_root_availability_metadata",
            digest: "e734af5d306f6ae72760181345fb83c95ab61f187c34ce395451f6409420349d",
            local_callees: &[],
            summary: "pure evidence-to-domain classifier",
        },
        AvailabilityFunctionContract {
            key: "fn:entry_reparse_evidence@cfg(windows)",
            digest: "4ca5bde2024d8f199722998902d250a73021a8b02af6d99b084b00f6beb77e6a",
            local_callees: &[
                "exact_attribute_tag_evidence",
                "has_reparse_point_attribute",
                "metadata_placeholder_state",
                "open_validation_handle",
                "reparse_evidence_from_attribute_tag",
            ],
            summary: "Windows no-recall exact reparse evidence",
        },
        AvailabilityFunctionContract {
            key: "fn:reparse_evidence_from_attribute_tag@cfg(windows)",
            digest: "92da8fc31572e847afb89313b979dd0bc647fe0071d3d9f2b97fe3cc2107e92a",
            local_callees: &[
                "cloud_placeholder_state_from_attribute_tag",
                "metadata_placeholder_state_from_attributes",
            ],
            summary: "value-only Cloud Files placeholder classifier",
        },
        AvailabilityFunctionContract {
            key: "fn:entry_reparse_evidence@cfg(not(windows))",
            digest: "d711ff9f03d84e3ecde39085b7f89a9f5ea15b5a7f3f0859289314f0cff27bc5",
            local_callees: &[],
            summary: "non-Windows symlink metadata classifier",
        },
        AvailabilityFunctionContract {
            key: "fn:has_reparse_point_attribute@cfg(windows)",
            digest: "dd17a38ef4fdaaf8b1810a3a67175aefbba9b14c82efdb8c6c6af57cfd7bc2e0",
            local_callees: &[],
            summary: "pure reparse attribute predicate",
        },
        AvailabilityFunctionContract {
            key: "fn:open_validation_handle@cfg(windows)",
            digest: "44d7242f19a18036e10766059ab0a5a2a29c0e2e480936c0b06b6e8dddd44750",
            local_callees: &[],
            summary: "single no-recall no-follow validation open",
        },
        AvailabilityFunctionContract {
            key: "fn:file_attribute_tag_info_from_handle@cfg(windows)",
            digest: "25a200cff9780d3bd0191293440d0dd3476fa5b81abaae54a07fa5abb78b1697",
            local_callees: &[],
            summary: "single held-handle attribute-tag query",
        },
        AvailabilityFunctionContract {
            key: "fn:exact_attribute_tag_evidence@cfg(windows)",
            digest: "2dca62957eb993dce90f4850ef887961f93c142d5bcd53074ef2bec643f28843",
            local_callees: &[
                "combine_attribute_tag_evidence",
                "exact_directory_entry_info",
                "file_attribute_tag_info_from_handle",
            ],
            summary: "bounded handle and exact-name evidence join",
        },
        AvailabilityFunctionContract {
            key: "fn:combine_attribute_tag_evidence@cfg(windows)",
            digest: "8e9d23548f8ea1319572889dc2dcb909ec1bc5dd3ac444dd0fd9e91898f79b5e",
            local_callees: &["has_reparse_point_attribute"],
            summary: "pure attribute-tag consistency check",
        },
        AvailabilityFunctionContract {
            key: "fn:exact_directory_entry_info@cfg(windows)",
            digest: "09307f6dbb73e76da442120b5bb3c0faa1d8fff797eb46324471d423e037cd1b",
            local_callees: &["windows_extended_path"],
            summary: "one exact-name FindFirstFile query, never directory traversal",
        },
        AvailabilityFunctionContract {
            key: "fn:windows_extended_path@cfg(windows)",
            digest: "8f83f6fee5c6f42eee51b70797dbaabb07210526b68a3d5896de87e26cf1771f",
            local_callees: &[],
            summary: "pure extended-path encoding",
        },
        AvailabilityFunctionContract {
            key: "fn:cloud_placeholder_state_from_attribute_tag@cfg(windows)",
            digest: "987adda8877e21f672fab81b7e046348233c511dc65af8ee5294afea184b08ca",
            local_callees: &[],
            summary: "single value-only Cloud Files state query",
        },
        AvailabilityFunctionContract {
            key: "fn:metadata_placeholder_state@cfg(windows)",
            digest: "b46121552ae9ba77ad4afa15fd11ba1786f894628ec41c3a8fbc19171e59d9ba",
            local_callees: &["metadata_placeholder_state_from_attributes"],
            summary: "Windows metadata attribute adapter",
        },
        AvailabilityFunctionContract {
            key: "fn:metadata_placeholder_state_from_attributes@cfg(windows)",
            digest: "f594f3e0debbad985c1313f3e93967484b692c281f45514b211c794f2548b406",
            local_callees: &[],
            summary: "pure Windows placeholder attribute classifier",
        },
        AvailabilityFunctionContract {
            key: "fn:metadata_placeholder_state@cfg(not(windows))",
            digest: "1cd8820f19100d444755a607fea66e4dc6c4ca023e63636e5855046b50464b99",
            local_callees: &[],
            summary: "pure non-Windows available-state adapter",
        },
    ];

    #[derive(Clone, Copy)]
    enum AvailabilitySupportSource {
        Local,
        Domain,
        MetadataDomain,
    }

    #[derive(Clone, Copy)]
    enum AvailabilitySupportLocator {
        Enum(&'static str),
        Struct(&'static str),
        Static(&'static str),
        UseContaining(&'static str),
    }

    #[derive(Clone, Copy)]
    struct AvailabilitySupportContract {
        key: &'static str,
        source: AvailabilitySupportSource,
        locator: AvailabilitySupportLocator,
        digest: &'static str,
        summary: &'static str,
    }

    const AVAILABILITY_SUPPORT_CONTRACTS: &[AvailabilitySupportContract] = &[
        AvailabilitySupportContract {
            key: "local-enum:RootAvailabilityMetadataEvidence",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::Enum("RootAvailabilityMetadataEvidence"),
            digest: "2f025d6b80b565fb4f05a0c3410a6ac3329ac413bb24bc609d9671e6166976a9",
            summary: "closed metadata evidence variants",
        },
        AvailabilitySupportContract {
            key: "local-enum:ReparseKind",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::Enum("ReparseKind"),
            digest: "1fb68b27cd00c3ce773b52952ab2624adde2d939add23a5695ad8e6e520528fb",
            summary: "closed reparse classification variants",
        },
        AvailabilitySupportContract {
            key: "local-struct:AttributeTagEvidence@cfg(windows)",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::Struct("AttributeTagEvidence"),
            digest: "7a70eeff2351cdc8b7d9f73801fda7029ebece1dbeff6aa80e0a855d30fc95f2",
            summary: "Windows attribute and tag evidence",
        },
        AvailabilitySupportContract {
            key: "local-static:ROOT_AVAILABILITY_METADATA_PROBE_COUNTS@cfg(test)",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::Static("ROOT_AVAILABILITY_METADATA_PROBE_COUNTS"),
            digest: "a98ecc9e6e29203a256cadd6a324a6aa70181908202ee8410522353e8ad4ac4d",
            summary: "test-only O(1) metadata probe counter initializer",
        },
        AvailabilitySupportContract {
            key: "local-use:std-fs-metadata",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining("std::fs::Metadata"),
            digest: "3a4df25d620e45592770fa629bd8f1d165af68d63419273cae864fbe9d4e9deb",
            summary: "canonical std filesystem evidence types",
        },
        AvailabilitySupportContract {
            key: "local-use:std-path",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining("std::path::Path"),
            digest: "68d1b9bd757116d2ecf03969184ff6f4b70aaf2b293d1872b18ecbf01043d831",
            summary: "canonical std path receiver types",
        },
        AvailabilitySupportContract {
            key: "local-use:windows-metadata-ext",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining("std::os::windows::fs::MetadataExt"),
            digest: "f1a479c53b8df108cb3173a92f4c07e604db60d70c5ae9206d8a88c781394e71",
            summary: "Windows metadata receiver provenance",
        },
        AvailabilitySupportContract {
            key: "local-use:windows-open-options-ext",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining(
                "std::os::windows::fs::OpenOptionsExt",
            ),
            digest: "aacaaa57203ca648c5e776f9a530c2f22a332cd56ac3e0d3debc21153bbf4489",
            summary: "Windows no-recall open receiver provenance",
        },
        AvailabilitySupportContract {
            key: "local-use:windows-raw-handle",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining("std::os::windows::io::AsRawHandle"),
            digest: "93d8e9d0b3716d82bbf290ce9642a685f3caf0da1d933f8ab53c544fdf29ddd9",
            summary: "Windows handle receiver provenance",
        },
        AvailabilitySupportContract {
            key: "local-use:windows-wide-path",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining("std::os::windows::ffi::OsStrExt"),
            digest: "dcc176b7663eec5da8f5993db5c6e02f558e9f756ed9c6c7f2409478cab06256",
            summary: "Windows UTF-16 path receiver provenance",
        },
        AvailabilitySupportContract {
            key: "local-use:cloud-filter-api",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining(
                "windows_sys::Win32::Storage::CloudFilters::CfGetPlaceholderStateFromAttributeTag",
            ),
            digest: "4a18add7072dfbee989b155989999399e24da6b61f777b169a640893b495bbb8",
            summary: "Cloud Files value-only state API",
        },
        AvailabilitySupportContract {
            key: "local-use:file-system-api",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining(
                "windows_sys::Win32::Storage::FileSystem::FindFirstFileW",
            ),
            digest: "b0c467ca86a60e4c30e7f84976f43ad0ace573657b2d0d24ca2c4c2d38b4246a",
            summary: "shared exact-name Win32 metadata, revision, and publication APIs",
        },
        AvailabilitySupportContract {
            key: "local-use:domain-availability-types",
            source: AvailabilitySupportSource::Local,
            locator: AvailabilitySupportLocator::UseContaining(
                "crate::domain::RootAvailabilityEvidence",
            ),
            digest: "fbe8afb4ad396f7dcce39c217abd1742da311b35225fb6bf9225f38ffaadee8b",
            summary: "shared Ame-owned availability and source-revision evidence types",
        },
        AvailabilitySupportContract {
            key: "domain-enum:LibraryRootAvailability",
            source: AvailabilitySupportSource::Domain,
            locator: AvailabilitySupportLocator::Enum("LibraryRootAvailability"),
            digest: "34e77ed57e8972cc61f1ed66358e789dda3c9b5f8ea68546b4db13f0df22b1f7",
            summary: "public root availability states",
        },
        AvailabilitySupportContract {
            key: "domain-enum:MetadataInventoryPlaceholderState",
            source: AvailabilitySupportSource::MetadataDomain,
            locator: AvailabilitySupportLocator::Enum("MetadataInventoryPlaceholderState"),
            digest: "567b7a9faf10527b498fe32ee7b8161002056323bce12b288e14a7a2af9cbdaa",
            summary: "public placeholder availability states",
        },
        AvailabilitySupportContract {
            key: "domain-struct:RootAvailabilityEvidence",
            source: AvailabilitySupportSource::Domain,
            locator: AvailabilitySupportLocator::Struct("RootAvailabilityEvidence"),
            digest: "481c2ec649040d9f896523d1691f2c4cd2749f275abb5d9dabf50b569efb0644",
            summary: "public root availability result shape",
        },
    ];

    struct AvailabilityFunctionRecord<'ast> {
        key: String,
        name: String,
        function: &'ast ItemFn,
    }

    #[derive(Default)]
    struct AvailabilityCallShapeCollector {
        calls: BTreeSet<String>,
        receivers: BTreeSet<String>,
    }

    impl<'ast> Visit<'ast> for AvailabilityCallShapeCollector {
        fn visit_expr_call(&mut self, call: &'ast ExprCall) {
            self.calls.insert(format!(
                "{}/{}",
                normalized_tokens(call.func.as_ref()),
                call.args.len()
            ));
            visit::visit_expr_call(self, call);
        }

        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            self.receivers.insert(format!(
                "{}=>{}/{}",
                normalized_tokens(call.receiver.as_ref()),
                call.method,
                call.args.len()
            ));
            visit::visit_expr_method_call(self, call);
        }
    }

    #[derive(Default)]
    struct AvailabilityMacroExpressionCollector {
        paths: Vec<String>,
    }

    impl<'ast> Visit<'ast> for AvailabilityMacroExpressionCollector {
        fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
            self.paths.push(normalized_tokens(&expression.mac.path));
            visit::visit_expr_macro(self, expression);
        }
    }

    struct RelevantImplVisitor {
        violations: Vec<String>,
    }

    impl<'ast> Visit<'ast> for RelevantImplVisitor {
        fn visit_item_impl(&mut self, implementation: &'ast ItemImpl) {
            let protected_types = [
                "AttributeTagEvidence",
                "LibraryRootAvailability",
                "MetadataInventoryPlaceholderState",
                "ReparseKind",
                "RootAvailabilityEvidence",
                "RootAvailabilityMetadataEvidence",
            ];
            let side_effect_traits = [
                "Add", "BitAnd", "BitOr", "BitXor", "Div", "Drop", "Mul", "Neg", "Not", "Rem",
                "Shl", "Shr", "Sub",
            ];
            let self_type = type_path_leaf(implementation.self_ty.as_ref());
            let trait_name = implementation
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .map(|segment| segment.ident.to_string());
            if self_type
                .as_ref()
                .is_some_and(|name| protected_types.contains(&name.as_str()))
                && trait_name
                    .as_ref()
                    .is_some_and(|name| side_effect_traits.contains(&name.as_str()))
            {
                self.violations.push(format!(
                    "relevant type {} gained explicit {} behavior",
                    self_type.expect("protected self type"),
                    trait_name.expect("side-effect trait")
                ));
            }
            visit::visit_item_impl(self, implementation);
        }
    }

    fn assert_no_relevant_availability_impls_in_crate() -> Result<(), String> {
        let source_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut pending = vec![source_root.clone()];
        let mut source_paths = Vec::new();
        while let Some(directory) = pending.pop() {
            let entries = fs::read_dir(&directory).map_err(|error| {
                format!(
                    "could not inspect Rust source directory {}: {error}",
                    directory.display()
                )
            })?;
            for entry in entries {
                let path = entry
                    .map_err(|error| format!("could not inspect Rust source entry: {error}"))?
                    .path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    source_paths.push(path);
                }
            }
        }
        source_paths.sort();
        let mut violations = Vec::new();
        for path in source_paths {
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            let file = syn::parse_file(&source)
                .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
            let mut visitor = RelevantImplVisitor {
                violations: Vec::new(),
            };
            visitor.visit_file(&file);
            let label = path.strip_prefix(&source_root).unwrap_or(&path).display();
            violations.extend(
                visitor
                    .violations
                    .into_iter()
                    .map(|violation| format!("{label}: {violation}")),
            );
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations.join("\n"))
        }
    }

    fn assert_availability_exact_source_contract(
        local_source: &str,
        domain_source: Option<&str>,
        metadata_domain_source: Option<&str>,
    ) -> Result<(), String> {
        assert_availability_exact_source_contract_with_environment(
            local_source,
            domain_source,
            metadata_domain_source,
            None,
            None,
        )
    }

    fn assert_availability_exact_source_contract_with_environment(
        local_source: &str,
        domain_source: Option<&str>,
        metadata_domain_source: Option<&str>,
        adapters_parent_source: Option<&str>,
        crate_parent_source: Option<&str>,
    ) -> Result<(), String> {
        let local_file = syn::parse_file(local_source)
            .map_err(|error| format!("availability source did not parse as Rust: {error}"))?;
        let domain_file = domain_source
            .map(syn::parse_file)
            .transpose()
            .map_err(|error| format!("domain source did not parse as Rust: {error}"))?;
        let metadata_domain_file = metadata_domain_source
            .map(syn::parse_file)
            .transpose()
            .map_err(|error| format!("metadata domain source did not parse as Rust: {error}"))?;
        let adapters_parent_file = adapters_parent_source
            .map(syn::parse_file)
            .transpose()
            .map_err(|error| format!("adapters parent source did not parse as Rust: {error}"))?;
        let crate_parent_file = crate_parent_source
            .map(syn::parse_file)
            .transpose()
            .map_err(|error| format!("crate parent source did not parse as Rust: {error}"))?;
        let mut errors = Vec::new();
        let mut protected_macro_names = BTreeSet::new();

        for (source_key, file, contracts) in [
            ("local", Some(&local_file), LOCAL_MODULE_CONTRACTS),
            (
                "adapters-parent",
                adapters_parent_file.as_ref(),
                ADAPTERS_PARENT_MODULE_CONTRACTS,
            ),
            (
                "crate-parent",
                crate_parent_file.as_ref(),
                CRATE_PARENT_MODULE_CONTRACTS,
            ),
            ("domain", domain_file.as_ref(), DOMAIN_MODULE_CONTRACTS),
            ("metadata-domain", metadata_domain_file.as_ref(), &[]),
        ] {
            if let Some(file) = file {
                validate_availability_module_topology(source_key, file, contracts, &mut errors);
            }
        }

        let functions = local_file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(function) => Some(AvailabilityFunctionRecord {
                    key: availability_function_key(function),
                    name: function.sig.ident.to_string(),
                    function,
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut functions_by_name = BTreeMap::<String, Vec<usize>>::new();
        for (index, function) in functions.iter().enumerate() {
            functions_by_name
                .entry(function.name.clone())
                .or_default()
                .push(index);
        }

        let mut queue = VecDeque::new();
        let mut closure = BTreeSet::new();
        for owner in [
            "inspect_root_availability",
            "probe_root_availability_metadata",
            "classify_root_availability_metadata",
        ] {
            match functions_by_name.get(owner).map(Vec::as_slice) {
                Some([index]) => queue.push_back(*index),
                Some(matches) => errors.push(format!(
                    "availability owner {owner} is ambiguous: {} top-level items",
                    matches.len()
                )),
                None => errors.push(format!("availability owner is missing: {owner}")),
            }
        }
        while let Some(index) = queue.pop_front() {
            if !closure.insert(index) {
                continue;
            }
            let mut shapes = AvailabilityCallShapeCollector::default();
            shapes.visit_block(&functions[index].function.block);
            for target in shapes.calls.iter().filter_map(|shape| {
                let target = shape.split('/').next()?;
                (!target.contains("::") && !target.contains(' ')).then_some(target)
            }) {
                if let Some(matches) = functions_by_name.get(target) {
                    for callee in matches {
                        queue.push_back(*callee);
                    }
                }
            }
        }

        let expected_functions = AVAILABILITY_FUNCTION_CONTRACTS
            .iter()
            .map(|contract| (contract.key, contract))
            .collect::<BTreeMap<_, _>>();
        let actual_keys = closure
            .iter()
            .map(|index| functions[*index].key.as_str())
            .collect::<BTreeSet<_>>();
        for contract in AVAILABILITY_FUNCTION_CONTRACTS {
            if !actual_keys.contains(contract.key) {
                errors.push(format!(
                    "availability contract item {} ({}) is missing",
                    contract.key, contract.summary
                ));
            }
        }
        for index in closure {
            let function = &functions[index];
            validate_availability_function_attributes(
                &function.function.attrs,
                &function.key,
                "local",
                &mut errors,
            );
            let mut macros = AvailabilityMacroExpressionCollector::default();
            macros.visit_block(&function.function.block);
            for path in macros.paths {
                if let Some(name) = path.rsplit("::").next() {
                    protected_macro_names.insert(name.to_owned());
                }
                errors.push(format!(
                    "availability contract item {} source local contains environment-rebindable macro expression {path}!",
                    function.key
                ));
            }
            let mut shapes = AvailabilityCallShapeCollector::default();
            shapes.visit_block(&function.function.block);
            let calls = shapes.calls.into_iter().collect::<Vec<_>>();
            let receivers = shapes.receivers.into_iter().collect::<Vec<_>>();
            let local_callees = calls
                .iter()
                .filter_map(|shape| shape.split('/').next())
                .filter(|target| functions_by_name.contains_key(*target))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let digest = token_digest(function.function);
            let Some(contract) = expected_functions.get(function.key.as_str()) else {
                errors.push(format!(
                    "unexpected availability call-closure item {}; digest={digest}; calls={calls:?}; receivers={receivers:?}",
                    function.key
                ));
                continue;
            };
            let expected_local_callees = contract
                .local_callees
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>();
            if digest != contract.digest || local_callees != expected_local_callees {
                errors.push(format!(
                    "availability contract item {} ({}) changed; expected_digest={}; actual_digest={digest}; expected_local_callees={expected_local_callees:?}; actual_local_callees={local_callees:?}; actual_calls={calls:?}; actual_receivers={receivers:?}",
                    contract.key, contract.summary, contract.digest
                ));
            }
        }

        for contract in AVAILABILITY_SUPPORT_CONTRACTS {
            let (source_key, source_file) = match contract.source {
                AvailabilitySupportSource::Local => ("local", Some(&local_file)),
                AvailabilitySupportSource::Domain => ("domain", domain_file.as_ref()),
                AvailabilitySupportSource::MetadataDomain => {
                    ("metadata-domain", metadata_domain_file.as_ref())
                }
            };
            let Some(source_file) = source_file else {
                errors.push(format!(
                    "availability support item {} ({}) has no source",
                    contract.key, contract.summary
                ));
                continue;
            };
            let matches = source_file
                .items
                .iter()
                .filter(|item| support_item_matches(item, contract.locator))
                .collect::<Vec<_>>();
            let [item] = matches.as_slice() else {
                errors.push(format!(
                    "availability support item {} ({}) resolved to {} items",
                    contract.key,
                    contract.summary,
                    matches.len()
                ));
                continue;
            };
            validate_availability_support_attributes(
                item,
                contract.key,
                source_key,
                &mut protected_macro_names,
                &mut errors,
            );
            let digest = token_digest(*item);
            if digest != contract.digest {
                errors.push(format!(
                    "availability support item {} ({}) changed; expected_digest={}; actual_digest={digest}",
                    contract.key, contract.summary, contract.digest
                ));
            }
        }

        for (source_key, file) in [
            ("local", Some(&local_file)),
            ("adapters-parent", adapters_parent_file.as_ref()),
            ("crate-parent", crate_parent_file.as_ref()),
            ("domain", domain_file.as_ref()),
            ("metadata-domain", metadata_domain_file.as_ref()),
        ] {
            if let Some(file) = file {
                validate_availability_macro_environment(
                    source_key,
                    file,
                    &protected_macro_names,
                    &mut errors,
                );
            }
        }

        for (label, file) in [
            ("local", Some(&local_file)),
            ("domain", domain_file.as_ref()),
            ("metadata-domain", metadata_domain_file.as_ref()),
        ] {
            let Some(file) = file else { continue };
            let mut visitor = RelevantImplVisitor {
                violations: Vec::new(),
            };
            visitor.visit_file(file);
            errors.extend(
                visitor
                    .violations
                    .into_iter()
                    .map(|violation| format!("{label} availability impl violation: {violation}")),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    fn validate_availability_module_topology(
        source_key: &str,
        file: &syn::File,
        contracts: &[AvailabilityModuleContract],
        errors: &mut Vec<String>,
    ) {
        for attribute in &file.attrs {
            errors.push(format!(
                "availability module topology source {source_key} has an unapproved file attribute {}",
                normalized_tokens(&attribute.meta)
            ));
        }

        let mut modules = BTreeMap::<String, Vec<&syn::ItemMod>>::new();
        for item in &file.items {
            match item {
                Item::Mod(module) => {
                    modules
                        .entry(module.ident.to_string())
                        .or_default()
                        .push(module);
                }
                Item::ExternCrate(item) => errors.push(format!(
                    "availability module topology source {source_key} has unapproved extern crate {}",
                    item.ident
                )),
                Item::Macro(item) => {
                    let attributes = item
                        .attrs
                        .iter()
                        .map(|attribute| compact_tokens(&attribute.meta))
                        .collect::<Vec<_>>();
                    let digest = token_digest(item);
                    if source_key != "local"
                        || !item.mac.path.is_ident("thread_local")
                        || attributes != ["cfg(all(windows,test))"]
                        || digest
                            != "3ad9146c8cc49b8c70e853426e4be6a0bbe97cd2314f141f9f2f81f97db19925"
                    {
                        errors.push(format!(
                            "availability module topology source {source_key} has unapproved item macro {}!; attributes={attributes:?}; digest={digest}",
                            normalized_tokens(&item.mac.path)
                        ));
                    }
                }
                Item::Verbatim(tokens) => errors.push(format!(
                    "availability module topology source {source_key} has unparsed top-level tokens {}",
                    normalized_tokens(tokens)
                )),
                _ => validate_availability_top_level_attributes(source_key, item, errors),
            }
        }

        let expected = contracts
            .iter()
            .map(|contract| (contract.name, contract))
            .collect::<BTreeMap<_, _>>();
        for contract in contracts {
            let Some(matches) = modules.get(contract.name) else {
                errors.push(format!(
                    "availability module topology source {source_key} is missing module {}",
                    contract.name
                ));
                continue;
            };
            let [module] = matches.as_slice() else {
                errors.push(format!(
                    "availability module topology source {source_key} resolves module {} to {} declarations",
                    contract.name,
                    matches.len()
                ));
                continue;
            };
            let visibility = compact_tokens(&module.vis);
            let attributes = module
                .attrs
                .iter()
                .map(|attribute| compact_tokens(&attribute.meta))
                .collect::<Vec<_>>();
            let expected_attributes = contract
                .attributes
                .iter()
                .map(|attribute| (*attribute).to_owned())
                .collect::<Vec<_>>();
            let is_inline = module.content.is_some();
            if visibility != contract.visibility
                || attributes != expected_attributes
                || is_inline != contract.is_inline
            {
                errors.push(format!(
                    "availability module topology source {source_key} module {} changed; expected_visibility={:?}; actual_visibility={visibility:?}; expected_attributes={expected_attributes:?}; actual_attributes={attributes:?}; expected_inline={}; actual_inline={is_inline}",
                    contract.name, contract.visibility, contract.is_inline
                ));
            }
        }
        for name in modules.keys() {
            if !expected.contains_key(name.as_str()) {
                errors.push(format!(
                    "availability module topology source {source_key} has unexpected nested or generated module {name}"
                ));
            }
        }
    }

    fn validate_availability_top_level_attributes(
        source_key: &str,
        item: &Item,
        errors: &mut Vec<String>,
    ) {
        let attributes = match item {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            _ => return,
        };
        let allowed_derives = [
            "Clone",
            "Copy",
            "Debug",
            "Default",
            "Eq",
            "Hash",
            "PartialEq",
        ];
        for attribute in attributes {
            if source_key == "domain"
                && matches!(
                    item,
                    Item::Struct(structure)
                        if structure.ident == "SourceRevisionEvidence"
                            && [
                                concat!(
                                    "doc = \" Filesystem-owned evidence that the bytes reachable ",
                                    "through a file identity may have changed.\""
                                ),
                                concat!(
                                    "doc = \" Windows ChangeTime is cheap change evidence, ",
                                    "not a content fingerprint.\""
                                ),
                            ]
                            .contains(&normalized_tokens(&attribute.meta).as_str())
                )
            {
                continue;
            }
            if attribute.path().is_ident("cfg") || attribute.path().is_ident("allow") {
                continue;
            }
            if attribute.path().is_ident("derive") {
                let parse_result = attribute.parse_nested_meta(|meta| {
                    let Some(name) = meta.path.get_ident().map(ToString::to_string) else {
                        errors.push(format!(
                            "availability module topology source {source_key} has a qualified derive attribute {}",
                            normalized_tokens(&meta.path)
                        ));
                        return Ok(());
                    };
                    if !allowed_derives.contains(&name.as_str()) {
                        errors.push(format!(
                            "availability module topology source {source_key} has an unapproved derive attribute {name}"
                        ));
                    }
                    Ok(())
                });
                if let Err(error) = parse_result {
                    errors.push(format!(
                        "availability module topology source {source_key} has an invalid derive attribute: {error}"
                    ));
                }
                continue;
            }
            errors.push(format!(
                "availability module topology source {source_key} has an unapproved top-level attribute {}",
                normalized_tokens(&attribute.meta)
            ));
        }
    }

    fn validate_availability_function_attributes(
        attributes: &[Attribute],
        item_key: &str,
        source_key: &str,
        errors: &mut Vec<String>,
    ) {
        for attribute in attributes {
            if !attribute.path().is_ident("cfg") {
                errors.push(format!(
                    "availability contract item {item_key} source {source_key} has environment-rebindable attribute {}",
                    normalized_tokens(&attribute.meta)
                ));
            }
        }
    }

    fn validate_availability_support_attributes(
        item: &Item,
        item_key: &str,
        source_key: &str,
        protected_macro_names: &mut BTreeSet<String>,
        errors: &mut Vec<String>,
    ) {
        let attributes = match item {
            Item::Enum(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            _ => {
                errors.push(format!(
                    "availability support item {item_key} source {source_key} has an unsupported item kind"
                ));
                return;
            }
        };
        let allowed_derives = ["Clone", "Copy", "Debug", "Eq", "Hash", "PartialEq"];
        for attribute in attributes {
            if attribute.path().is_ident("cfg") {
                continue;
            }
            if attribute.path().is_ident("derive") {
                let parse_result = attribute.parse_nested_meta(|meta| {
                    let Some(name) = meta.path.get_ident().map(ToString::to_string) else {
                        errors.push(format!(
                            "availability support item {item_key} source {source_key} has a qualified derive macro {}",
                            normalized_tokens(&meta.path)
                        ));
                        return Ok(());
                    };
                    if !allowed_derives.contains(&name.as_str()) {
                        errors.push(format!(
                            "availability support item {item_key} source {source_key} has an unapproved derive macro {name}"
                        ));
                    }
                    protected_macro_names.insert(name);
                    Ok(())
                });
                if let Err(error) = parse_result {
                    errors.push(format!(
                        "availability support item {item_key} source {source_key} has an invalid derive list: {error}"
                    ));
                }
                continue;
            }
            errors.push(format!(
                "availability support item {item_key} source {source_key} has environment-rebindable attribute {}",
                normalized_tokens(&attribute.meta)
            ));
        }
    }

    fn validate_availability_macro_environment(
        source_key: &str,
        file: &syn::File,
        protected_macro_names: &BTreeSet<String>,
        errors: &mut Vec<String>,
    ) {
        if protected_macro_names.is_empty() {
            return;
        }
        for attribute in &file.attrs {
            if attribute.path().is_ident("macro_use") {
                errors.push(format!(
                    "availability macro environment source {source_key} item file-attribute:macro_use can import protected macros"
                ));
            }
        }
        for item in &file.items {
            let attributes = match item {
                Item::Const(item) => &item.attrs,
                Item::Enum(item) => &item.attrs,
                Item::ExternCrate(item) => &item.attrs,
                Item::Fn(item) => &item.attrs,
                Item::ForeignMod(item) => &item.attrs,
                Item::Impl(item) => &item.attrs,
                Item::Macro(item) => &item.attrs,
                Item::Mod(item) => &item.attrs,
                Item::Static(item) => &item.attrs,
                Item::Struct(item) => &item.attrs,
                Item::Trait(item) => &item.attrs,
                Item::TraitAlias(item) => &item.attrs,
                Item::Type(item) => &item.attrs,
                Item::Union(item) => &item.attrs,
                Item::Use(item) => &item.attrs,
                Item::Verbatim(_) => continue,
                _ => continue,
            };
            if attributes
                .iter()
                .any(|attribute| attribute.path().is_ident("macro_use"))
            {
                errors.push(format!(
                    "availability macro environment source {source_key} item attribute:macro_use can import protected macros"
                ));
            }
            if let Item::Macro(item_macro) = item
                && item_macro.mac.path.is_ident("macro_rules")
                && let Some(name) = item_macro.ident.as_ref().map(ToString::to_string)
                && protected_macro_names.contains(&name)
            {
                errors.push(format!(
                    "availability macro environment source {source_key} item macro_rules:{name} can rebind protected macro {name}"
                ));
            }
            if let Item::Use(item_use) = item {
                let mut bindings = Vec::new();
                let mut has_glob = false;
                collect_availability_use_bindings(&item_use.tree, &mut bindings, &mut has_glob);
                if has_glob {
                    errors.push(format!(
                        "availability macro environment source {source_key} item use:glob can import protected macros"
                    ));
                }
                for binding in bindings {
                    if protected_macro_names.contains(&binding) {
                        errors.push(format!(
                            "availability macro environment source {source_key} item use:{binding} can rebind protected macro {binding}"
                        ));
                    }
                }
            }
        }
    }

    fn collect_availability_use_bindings(
        tree: &UseTree,
        bindings: &mut Vec<String>,
        has_glob: &mut bool,
    ) {
        match tree {
            UseTree::Path(path) => {
                collect_availability_use_bindings(path.tree.as_ref(), bindings, has_glob);
            }
            UseTree::Name(name) => bindings.push(name.ident.to_string()),
            UseTree::Rename(rename) => bindings.push(rename.rename.to_string()),
            UseTree::Glob(_) => *has_glob = true,
            UseTree::Group(group) => {
                for item in &group.items {
                    collect_availability_use_bindings(item, bindings, has_glob);
                }
            }
        }
    }

    fn availability_function_key(function: &ItemFn) -> String {
        format!("fn:{}{}", function.sig.ident, cfg_suffix(&function.attrs))
    }

    fn cfg_suffix(attributes: &[Attribute]) -> String {
        let cfgs = attributes
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg"))
            .map(|attribute| compact_tokens(&attribute.meta))
            .collect::<Vec<_>>();
        if cfgs.is_empty() {
            String::new()
        } else {
            format!("@{}", cfgs.join("&"))
        }
    }

    fn compact_tokens(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }

    fn normalized_tokens(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string()
    }

    fn token_digest(tokens: &impl ToTokens) -> String {
        blake3::hash(normalized_tokens(tokens).as_bytes())
            .to_hex()
            .to_string()
    }

    fn support_item_matches(item: &Item, locator: AvailabilitySupportLocator) -> bool {
        match (locator, item) {
            (AvailabilitySupportLocator::Enum(expected), Item::Enum(item)) => {
                item.ident == expected
            }
            (AvailabilitySupportLocator::Struct(expected), Item::Struct(item)) => {
                item.ident == expected
            }
            (AvailabilitySupportLocator::Static(expected), Item::Static(item)) => {
                item.ident == expected
            }
            (AvailabilitySupportLocator::UseContaining(expected), Item::Use(item)) => {
                let mut paths = Vec::new();
                flatten_use_tree(&item.tree, Vec::new(), &mut paths);
                paths.iter().any(|path| path == expected)
            }
            _ => false,
        }
    }

    fn flatten_use_tree(tree: &UseTree, prefix: Vec<String>, paths: &mut Vec<String>) {
        match tree {
            UseTree::Path(path) => {
                let mut next = prefix;
                next.push(path.ident.to_string());
                flatten_use_tree(path.tree.as_ref(), next, paths);
            }
            UseTree::Name(name) => {
                let mut path = prefix;
                if name.ident != "self" {
                    path.push(name.ident.to_string());
                }
                paths.push(path.join("::"));
            }
            UseTree::Rename(rename) => {
                let mut path = prefix;
                path.push(rename.ident.to_string());
                paths.push(path.join("::"));
            }
            UseTree::Glob(_) => {
                let mut path = prefix;
                path.push("*".to_owned());
                paths.push(path.join("::"));
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    flatten_use_tree(item, prefix.clone(), paths);
                }
            }
        }
    }

    fn type_path_leaf(value: &Type) -> Option<String> {
        let Type::Path(path) = value else {
            return None;
        };
        path.path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
    }
}
