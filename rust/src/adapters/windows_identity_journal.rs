use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::domain::{
    JournalFileReference, LibraryRootGeneration, PersistentJournalVolumeIdentity, ScanError,
};
use crate::journal_broker::{
    BrokerResponse, CallerClaim, JournalCapability, PersistentChangeJournal,
    PersistentChangeJournalConnection, PersistentChangeJournalOperationError,
    PersistentChangeJournalRead, PersistentChangeJournalSession, QueryJournalRequest,
    ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding,
    RootCapability,
};
use crate::ports::{
    IdentityJournalPage, IdentityJournalRead, IdentityJournalRecord, IdentityJournalResolution,
    IdentityJournalRoot, PersistentIdentityJournalSource, identity_for_reference,
};
use crate::windows_usn::{UsnParseError, decode_journal_buffer};

mod io;
mod native;
use native::NativeRoot;

const READ_UNPRIVILEGED_USN_JOURNAL: u32 = 0x0009_03ab;
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ROOTS: usize = 8;
const MAX_REFERENCES: usize = 256;
const MAX_LOCATIONS: usize = 4_096;
type RootKey = (String, LibraryRootGeneration);

pub(crate) fn production_factory() -> Arc<dyn PersistentChangeJournal> {
    Arc::new(IdentityJournalFactory)
}

struct IdentityJournalFactory;
impl PersistentChangeJournal for IdentityJournalFactory {
    fn connect(&self) -> PersistentChangeJournalConnection {
        PersistentChangeJournalConnection::Connected(Arc::new(IdentitySession {
            source: Arc::new(IdentitySource {
                roots: Mutex::new(HashMap::new()),
                probed: Mutex::new(HashMap::new()),
                closed: AtomicBool::new(false),
            }),
        }))
    }
}

struct Registration {
    root: Arc<NativeRoot>,
    caller: CallerClaim,
    capability: RootCapability,
    last_used: Instant,
}

struct IdentitySource {
    roots: Mutex<HashMap<RootKey, Registration>>,
    probed: Mutex<HashMap<RootKey, Instant>>,
    closed: AtomicBool,
}

struct IdentitySession {
    source: Arc<IdentitySource>,
}

impl IdentitySource {
    fn root(&self, key: &IdentityJournalRoot) -> Result<Arc<NativeRoot>, ScanError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(invalid("identity_journal_closed"));
        }
        self.roots
            .lock()
            .map_err(|_| invalid("identity_journal_registry_unavailable"))?
            .get_mut(&(key.root_id.clone(), key.root_generation))
            .map(|entry| {
                entry.last_used = Instant::now();
                Arc::clone(&entry.root)
            })
            .ok_or_else(|| invalid("identity_journal_root_unregistered"))
    }
}

impl PersistentChangeJournalSession for IdentitySession {
    fn identity_source(&self) -> Option<Arc<dyn PersistentIdentityJournalSource>> {
        Some(self.source.clone())
    }

    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        request
            .validate()
            .map_err(|_| PersistentChangeJournalOperationError::InvalidRequest)?;
        if self.source.closed.load(Ordering::Acquire) {
            return Err(PersistentChangeJournalOperationError::Closed);
        }
        if request.caller.process_id != std::process::id() {
            return Err(PersistentChangeJournalOperationError::InvalidRequest);
        }
        let generation = LibraryRootGeneration::new(request.root.root_generation)
            .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
        let key = (request.root.root_id.clone(), generation);
        {
            let mut probed = self
                .source
                .probed
                .lock()
                .map_err(|_| PersistentChangeJournalOperationError::TransportUnavailable)?;
            probed.retain(|_, at| at.elapsed() < Duration::from_secs(60));
            if probed.len() >= 64
                && !probed.contains_key(&key)
                && let Some(oldest) = probed
                    .iter()
                    .min_by_key(|(_, at)| **at)
                    .map(|(key, _)| key.clone())
            {
                probed.remove(&oldest);
            }
            probed.insert(key.clone(), Instant::now());
        }
        {
            let mut roots = self
                .source
                .roots
                .lock()
                .map_err(|_| PersistentChangeJournalOperationError::TransportUnavailable)?;
            roots.retain(|existing_key, entry| {
                (existing_key.0 != key.0 || existing_key.1 == key.1)
                    && entry.last_used.elapsed() < Duration::from_secs(60)
            });
            if let Some(existing) = roots.get_mut(&key) {
                if existing.root.authorization == request.root && existing.caller == request.caller
                {
                    existing.last_used = Instant::now();
                    return Ok(existing.capability);
                }
                return Err(PersistentChangeJournalOperationError::InvalidRequest);
            }
            if roots.len() >= MAX_ROOTS
                && let Some(oldest) = roots
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_used)
                    .map(|(key, _)| key.clone())
            {
                roots.remove(&oldest);
            }
        }
        let root = Arc::new(
            NativeRoot::open(&request.root)
                .map_err(|_| PersistentChangeJournalOperationError::InvalidRequest)?,
        );
        let mut digest = blake3::Hasher::new();
        digest.update(&request.caller.client_instance);
        digest.update(request.root.root_id.as_bytes());
        digest.update(&request.root.root_generation.to_le_bytes());
        digest.update(&request.root.root_identity);
        let capability = RootCapability(*digest.finalize().as_bytes());
        let mut roots = self
            .source
            .roots
            .lock()
            .map_err(|_| PersistentChangeJournalOperationError::TransportUnavailable)?;
        if self.source.closed.load(Ordering::Acquire) {
            return Err(PersistentChangeJournalOperationError::Closed);
        }
        if roots.len() >= MAX_ROOTS || roots.contains_key(&key) {
            return Err(PersistentChangeJournalOperationError::CapacityExceeded);
        }
        roots.insert(
            key,
            Registration {
                root,
                caller: request.caller,
                capability,
                last_used: Instant::now(),
            },
        );
        Ok(capability)
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        request
            .validate()
            .map_err(|_| PersistentChangeJournalOperationError::InvalidRequest)?;
        let generation = LibraryRootGeneration::new(request.root.root_generation)
            .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
        let root = {
            let mut roots = self
                .source
                .roots
                .lock()
                .map_err(|_| PersistentChangeJournalOperationError::TransportUnavailable)?;
            let registered = roots
                .get_mut(&(request.root.root_id.clone(), generation))
                .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
            if registered.caller != request.caller
                || registered.capability != request.root_capability
                || registered.root.authorization != request.root
            {
                return Err(PersistentChangeJournalOperationError::InvalidRequest);
            }
            registered.last_used = Instant::now();
            Arc::clone(&registered.root)
        };
        let boundary = match root.query(
            &AtomicBool::new(false),
            Instant::now() + Duration::from_millis(u64::from(request.timeout_ms)),
        ) {
            Ok(boundary) => boundary,
            Err(error) if error.code == "identity_journal_not_supported" => {
                return Ok(BrokerResponse::Journal {
                    request_id: 1,
                    binding: ResponseBinding::from_request(&request.caller, &request.root),
                    capability: JournalCapability::LiveOnly,
                    journal_id: None,
                    first_usn: None,
                    next_usn: None,
                });
            }
            Err(_) => return Err(PersistentChangeJournalOperationError::TransportUnavailable),
        };
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(boundary.0),
            first_usn: Some(boundary.1),
            next_usn: Some(boundary.2),
        })
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        _request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.source.closed.store(true, Ordering::Release);
        self.source
            .roots
            .lock()
            .map_err(|_| PersistentChangeJournalOperationError::TransportUnavailable)?
            .clear();
        Ok(())
    }
}

impl PersistentIdentityJournalSource for IdentitySource {
    fn can_probe_baseline(&self, root: &IdentityJournalRoot) -> bool {
        self.probed.lock().is_ok_and(|mut probed| {
            probed.retain(|_, at| at.elapsed() < Duration::from_secs(60));
            if let Some(at) = probed.get_mut(&(root.root_id.clone(), root.root_generation)) {
                *at = Instant::now();
                false
            } else {
                true
            }
        })
    }

    fn volume_identity(
        &self,
        root: &IdentityJournalRoot,
    ) -> Result<PersistentJournalVolumeIdentity, ScanError> {
        Ok(self.root(root)?.volume.clone())
    }

    fn read_volume(
        &self,
        request: &IdentityJournalRead,
        cancelled: &AtomicBool,
    ) -> Result<IdentityJournalPage, ScanError> {
        if request.start_usn < 0
            || request.end_usn <= request.start_usn
            || !(1..=256).contains(&request.max_records)
        {
            return Err(invalid("identity_journal_range_invalid"));
        }
        let root = {
            let roots = self
                .roots
                .lock()
                .map_err(|_| invalid("identity_journal_registry_unavailable"))?;
            roots
                .values()
                .filter(|entry| entry.root.volume == request.volume)
                .max_by_key(|entry| entry.last_used)
                .map(|entry| Arc::clone(&entry.root))
                .ok_or_else(|| invalid("identity_journal_volume_mismatch"))?
        };
        let deadline = Instant::now() + READ_TIMEOUT;
        let before = root.query(cancelled, deadline)?;
        if before.0 != request.journal_id
            || before.1 > request.start_usn
            || before.2 < request.end_usn
        {
            return Err(invalid("identity_journal_discontinuous"));
        }
        let mut input = [0_u8; 40];
        input[..8].copy_from_slice(&request.start_usn.to_le_bytes());
        input[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        input[32..40].copy_from_slice(&request.journal_id.to_le_bytes());
        let mut buffer = vec![0_u8; 64 * 1_024];
        let count = io::control(
            &root.volume_file,
            READ_UNPRIVILEGED_USN_JOURNAL,
            &input,
            &mut buffer,
            cancelled,
            deadline,
        )?;
        let (next, records) = decode_journal_buffer(&buffer[..count], |record| {
            if record.usn < 0 {
                return Err(UsnParseError::RecordMalformed);
            }
            Ok(IdentityJournalRecord {
                file_reference: JournalFileReference::from_bytes(&record.file_reference.bytes())
                    .map_err(|_| UsnParseError::RecordMalformed)?,
                parent_reference: JournalFileReference::from_bytes(
                    &record.parent_reference.bytes(),
                )
                .map_err(|_| UsnParseError::RecordMalformed)?,
                usn: record.usn,
                is_directory: record.file_attributes
                    & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY
                    != 0,
                is_deleted: record.reason
                    & windows_sys::Win32::System::Ioctl::USN_REASON_FILE_DELETE
                    != 0,
            })
        })
        .map_err(|_| invalid("identity_journal_record_invalid"))?;
        if next <= request.start_usn
            || records.windows(2).any(|pair| pair[0].usn >= pair[1].usn)
            || records
                .iter()
                .any(|record| record.usn < request.start_usn || record.usn >= next)
        {
            return Err(invalid("identity_journal_discontinuous"));
        }
        let mut records: Vec<_> = records
            .into_iter()
            .filter(|record| record.usn < request.end_usn)
            .collect();
        let mut covered = next.min(request.end_usn);
        if records.len() > request.max_records as usize {
            covered = records[request.max_records as usize].usn;
            records.truncate(request.max_records as usize);
        }
        let after = root.query(cancelled, deadline)?;
        if after.0 != before.0 || after.1 > request.start_usn || after.2 < request.end_usn {
            return Err(invalid("identity_journal_discontinuous"));
        }
        Ok(IdentityJournalPage {
            records,
            covered_until_usn: covered,
        })
    }

    fn current_locations(
        &self,
        root: &IdentityJournalRoot,
        references: &[JournalFileReference],
        cancelled: &AtomicBool,
    ) -> Result<IdentityJournalResolution, ScanError> {
        if references.len() > MAX_REFERENCES {
            return Err(invalid("identity_journal_resolution_capacity"));
        }
        let root = self.root(root)?;
        let _guard = root.guard()?;
        let deadline = Instant::now() + READ_TIMEOUT;
        let mut resolution = IdentityJournalResolution::default();
        for reference in references {
            io::check_active(cancelled, deadline)?;
            match root.locations(reference) {
                Ok(Some(locations)) => resolution.locations.extend(locations),
                Ok(None) => resolution
                    .unresolved
                    .push(identity_for_reference(root.volume.volume_serial, reference)),
                Err(error) if error.code == "identity_journal_identity_access_denied" => {
                    resolution
                        .access_denied
                        .push(identity_for_reference(root.volume.volume_serial, reference));
                }
                Err(error) => return Err(error),
            }
            if resolution.locations.len() > MAX_LOCATIONS {
                return Err(invalid("identity_journal_resolution_capacity"));
            }
        }
        let _final_guard = root.guard()?;
        if self.closed.load(Ordering::Acquire) {
            return Err(invalid("identity_journal_closed"));
        }
        Ok(resolution)
    }
}

fn invalid(code: &str) -> ScanError {
    ScanError::new(
        code,
        "The bounded identity journal evidence could not be verified",
    )
}
