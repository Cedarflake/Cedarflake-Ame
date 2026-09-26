use std::collections::HashMap;
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, DuplicateHandle, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GENERIC_READ, HANDLE,
    INVALID_HANDLE_VALUE, LUID,
};
use windows_sys::Win32::Security::{
    DuplicateTokenEx, GetLengthSid, GetTokenInformation, ImpersonateLoggedOnUser, IsValidSid,
    RevertToSelf, SecurityImpersonation, TOKEN_DUPLICATE, TOKEN_ELEVATION, TOKEN_IMPERSONATE,
    TOKEN_QUERY, TOKEN_STATISTICS, TOKEN_TYPE, TOKEN_USER, TokenElevation, TokenImpersonation,
    TokenImpersonationLevel, TokenPrimary, TokenSessionId, TokenStatistics, TokenType, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_CASE_SENSITIVE_INFO,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED, FILE_ID_INFO,
    FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_STANDARD_INFO, FileAttributeTagInfo, FileCaseSensitiveInfo, FileIdInfo,
    FileStandardInfo, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
    GetVolumeInformationByHandleW, OPEN_EXISTING, VOLUME_NAME_GUID,
};
use windows_sys::Win32::System::Pipes::ImpersonateNamedPipeClient;
use windows_sys::Win32::System::SystemServices::FILE_CS_FLAG_CASE_SENSITIVE_DIR;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, OpenProcess, OpenProcessToken, OpenThreadToken,
    PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::{
    AuthenticatedConnection, AuthenticatedPipeFacts, AuthorizedPinnedRoot, AuthorizerError,
    BackendJournalCandidate, CallerAuthorization, CallerAuthorizer, DeclaredRootAuthorization,
    PinnedBrokerRoot, RequestContext,
};
use crate::journal_broker::windows::service_identity::VerifiedClientProcessIdentity;
use crate::journal_broker::wire::{
    CallerClaim, NameComparisonSemantics, RootAuthorization, RootCapability, RootRelativeScope,
    normalize_backend_path, scope_under_root,
};

const MAX_TOKEN_INFORMATION_BYTES: usize = 64 * 1024;
const MAX_SID_BYTES: usize = 256;
const MAX_WIN32_PATH_UNITS: usize = 32_767;
const MAX_REGISTERED_ROOTS_PER_CONNECTION: usize = 8;
const MAX_REGISTERED_ROOTS_GLOBAL: usize = 16;
const ROOT_CAPABILITY_LIFETIME: Duration = Duration::from_secs(120);
const VOLUME_OPEN_FLAGS: u32 = FILE_FLAG_OVERLAPPED;
const ROOT_DUPLICATED_ACCESS: u32 = FILE_READ_ATTRIBUTES;

pub(in crate::journal_broker) struct WindowsPipeAuthorizer {
    security: Option<Arc<ConnectionSecurity>>,
}

#[derive(Clone)]
pub(in crate::journal_broker) struct WindowsRootRegistrationBudget(Arc<RootBudgetState>);

struct RootBudgetState {
    active: Mutex<usize>,
    limit: usize,
}

struct RootRegistrationPermit {
    budget: Arc<RootBudgetState>,
}

impl WindowsRootRegistrationBudget {
    pub(in crate::journal_broker) fn production() -> Self {
        Self::with_limit(MAX_REGISTERED_ROOTS_GLOBAL)
    }

    fn with_limit(limit: usize) -> Self {
        Self(Arc::new(RootBudgetState {
            active: Mutex::new(0),
            limit,
        }))
    }

    fn try_acquire(&self) -> Result<RootRegistrationPermit, AuthorizerError> {
        let mut active = self
            .0
            .active
            .lock()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        if *active >= self.0.limit {
            return Err(AuthorizerError::RootUnauthorized);
        }
        *active += 1;
        Ok(RootRegistrationPermit {
            budget: Arc::clone(&self.0),
        })
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.0.active.lock().map_or(usize::MAX, |active| *active)
    }
}

impl Drop for RootRegistrationPermit {
    fn drop(&mut self) {
        if let Ok(mut active) = self.budget.active.lock() {
            *active = active.saturating_sub(1);
        }
    }
}

pub(in crate::journal_broker) enum ClientBinaryAdmission {
    Production(VerifiedClientProcessIdentity),
    DisposableTest,
}

pub(in crate::journal_broker) struct ConnectedPipeAdmission {
    connection_id: [u8; 16],
    connection_generation: u64,
    connection_nonce: u64,
    observed_process_id: u32,
    observed_session_id: u32,
    client_binary: ClientBinaryAdmission,
}

impl ConnectedPipeAdmission {
    pub(in crate::journal_broker) fn new(
        connection_id: [u8; 16],
        connection_generation: u64,
        connection_nonce: u64,
        observed_process_id: u32,
        observed_session_id: u32,
        client_binary: ClientBinaryAdmission,
    ) -> Self {
        Self {
            connection_id,
            connection_generation,
            connection_nonce,
            observed_process_id,
            observed_session_id,
            client_binary,
        }
    }
}

impl WindowsPipeAuthorizer {
    pub(in crate::journal_broker) fn from_connected_pipe(
        pipe: HANDLE,
        admission: ConnectedPipeAdmission,
        root_budget: WindowsRootRegistrationBudget,
    ) -> Result<(AuthenticatedConnection, Self), WindowsSecurityError> {
        let ConnectedPipeAdmission {
            connection_id,
            connection_generation,
            connection_nonce,
            observed_process_id,
            observed_session_id,
            client_binary,
        } = admission;
        if pipe.is_null()
            || connection_id == [0; 16]
            || connection_generation == 0
            || connection_nonce == 0
            || observed_process_id == 0
        {
            return Err(WindowsSecurityError::CallerRejected);
        }
        let token = capture_named_pipe_client_token(pipe)?;
        let primary = query_primary_token_evidence(observed_process_id)?;
        let evidence = query_token_evidence(&token, primary.is_elevated)?;
        if evidence.session_id != observed_session_id || !primary.matches_impersonation(&evidence) {
            return Err(WindowsSecurityError::CallerRejected);
        }
        let client_binary_identity = match client_binary {
            ClientBinaryAdmission::Production(identity) => {
                if !identity.matches_pipe_token(
                    observed_process_id,
                    observed_session_id,
                    &evidence.user_sid,
                    luid_bytes(evidence.authentication_id),
                    evidence.is_elevated,
                ) {
                    return Err(WindowsSecurityError::CallerRejected);
                }
                identity.opaque_binary_identity()
            }
            ClientBinaryAdmission::DisposableTest => disposable_client_identity(
                connection_id,
                connection_generation,
                connection_nonce,
                observed_process_id,
                observed_session_id,
            ),
        };
        let namespace = token_binding(
            &evidence,
            connection_id,
            connection_generation,
            connection_nonce,
            observed_process_id,
            observed_session_id,
            client_binary_identity,
        );
        let authenticated = AuthenticatedConnection::from_pipe_token(
            connection_id,
            connection_generation,
            AuthenticatedPipeFacts {
                opaque_token_binding: namespace,
                client_binary_identity,
                process_id: observed_process_id,
                session_id: observed_session_id,
                token_type: evidence.token_type,
                impersonation_level: evidence.impersonation_level,
                is_elevated: evidence.is_elevated,
            },
        )
        .map_err(|_| WindowsSecurityError::CallerRejected)?;
        Ok((
            authenticated,
            Self {
                security: Some(Arc::new(ConnectionSecurity {
                    token,
                    namespace,
                    client_binary_identity,
                    observed_process_id,
                    observed_session_id,
                    root_budget,
                    registrations: Mutex::new(HashMap::new()),
                })),
            },
        ))
    }

    #[cfg(test)]
    pub(in crate::journal_broker) const fn fail_closed_for_test() -> Self {
        Self { security: None }
    }

    pub(in crate::journal_broker) fn backend_security(
        &self,
    ) -> Result<WindowsBrokerSecurity, WindowsSecurityError> {
        self.security
            .as_ref()
            .cloned()
            .map(WindowsBrokerSecurity)
            .ok_or(WindowsSecurityError::CallerRejected)
    }
}

impl CallerAuthorizer for WindowsPipeAuthorizer {
    fn reap_expired(&self, now: Instant) {
        if let Some(security) = &self.security {
            security.reap_expired(now);
        }
    }

    fn verify_claim(
        &self,
        authenticated: &AuthenticatedConnection,
        claim: &CallerClaim,
    ) -> Result<CallerAuthorization, AuthorizerError> {
        let security = self
            .security
            .as_ref()
            .ok_or(AuthorizerError::CallerRejected)?;
        if authenticated.opaque_token_binding != security.namespace
            || authenticated.observed_client_binary_identity != security.client_binary_identity
            || authenticated.observed_process_id != security.observed_process_id
            || authenticated.observed_session_id != security.observed_session_id
        {
            return Err(AuthorizerError::CallerRejected);
        }
        CallerAuthorization::from_verified_claim(authenticated, claim)
    }

    fn register_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        client_root_handle: u64,
        context: &mut RequestContext<'_>,
    ) -> Result<RootCapability, AuthorizerError> {
        if client_root_handle == 0 {
            return Err(AuthorizerError::RootUnauthorized);
        }
        let security = self
            .security
            .as_ref()
            .ok_or(AuthorizerError::RootUnauthorized)?;
        if caller.opaque_namespace != security.namespace {
            return Err(AuthorizerError::RootUnauthorized);
        }
        context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        let mut pinned = security
            .duplicate_and_pin_client_root(client_root_handle, root)
            .map_err(|error| match error {
                WindowsSecurityError::UnsupportedFilesystem => {
                    AuthorizerError::UnsupportedFilesystem
                }
                _ => AuthorizerError::RootIdentityMismatch,
            })?;
        pinned.volume_handle = Some(
            open_volume_handle(&pinned.volume_open_path)
                .map_err(|_| AuthorizerError::RootIdentityMismatch)?,
        );
        let mut hasher = blake3::Hasher::new_keyed(&security.namespace);
        hasher.update(b"cedarflake-ame-root-capability-v1\0");
        hasher.update(&caller.connection_id);
        hasher.update(&caller.connection_generation.to_le_bytes());
        hasher.update(&caller.client_instance);
        hasher.update(&security.client_binary_identity);
        hasher.update(root.root_id.as_bytes());
        hasher.update(&root.root_generation.to_le_bytes());
        hasher.update(root.volume_id.as_bytes());
        hasher.update(&root.root_identity);
        hasher.update(&client_root_handle.to_le_bytes());
        let capability = RootCapability(*hasher.finalize().as_bytes());
        let key = RootKey {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
        };
        insert_registration(security, key, capability, Arc::new(pinned))?;
        context
            .check()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        Ok(capability)
    }

    fn authorize_registered_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        capability: RootCapability,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        let security = self
            .security
            .as_ref()
            .ok_or(AuthorizerError::RootUnauthorized)?;
        let key = RootKey {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
        };
        let mut registrations = security
            .registrations
            .lock()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        reap_registrations(&mut registrations, Instant::now());
        let pinned = registrations
            .get(&key)
            .filter(|registration| registration.capability == capability)
            .map(|registration| Arc::clone(&registration.pinned))
            .ok_or(AuthorizerError::RootUnauthorized)?;
        drop(registrations);
        context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        if pinned.volume_id != root.volume_id
            || pinned.root_identity != root.root_identity
            || pinned.canonical_root_utf16 != root.canonical_root_utf16
        {
            return Err(AuthorizerError::RootIdentityMismatch);
        }
        pinned
            .revalidate_identity()
            .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let proof = PinnedBrokerRoot::from_caller_pinned_handle(
            &pinned.volume_id,
            &pinned.root_identity,
            &pinned.canonical_root_utf16,
            super::contracts::RootNameSemanticsProof::from_pinned_directory_handles(
                &pinned.component_semantics,
            )?,
        )
        .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let declared = DeclaredRootAuthorization::after_access_check(caller, root)?;
        AuthorizedPinnedRoot::after_identity_check(caller, &declared, proof)
    }

    fn filter_disclosable_candidates(
        &self,
        root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError> {
        let security = self
            .security
            .as_ref()
            .ok_or(AuthorizerError::RootUnauthorized)?;
        if root.caller.opaque_namespace != security.namespace {
            return Err(AuthorizerError::RootUnauthorized);
        }
        let normalized_root = normalize_backend_path(&root.pinned_root.canonical_root_utf16)
            .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let comparisons = root.pinned_root.name_semantics.components();
        let root_path = String::from_utf16(&root.pinned_root.canonical_root_utf16)
            .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let mut disclosable = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            context
                .bounded_step()
                .map_err(|_| AuthorizerError::RootUnauthorized)?;
            if candidate.volume_id != root.pinned_root.volume_id {
                continue;
            }
            let mut scopes = Vec::with_capacity(2);
            for path in std::iter::once(Some(&candidate.absolute_path_utf16))
                .chain(std::iter::once(
                    candidate.previous_absolute_path_utf16.as_ref(),
                ))
                .flatten()
            {
                let normalized = normalize_backend_path(path)
                    .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
                if let Some(scope) = scope_under_root(&normalized, &normalized_root, comparisons)
                    .map_err(|_| AuthorizerError::RootIdentityMismatch)?
                {
                    scopes.push(scope);
                }
            }
            if scopes.is_empty() {
                continue;
            }
            let visible = security
                .impersonate(|| {
                    for scope in &scopes {
                        if !probe_disclosable_scope(
                            &root_path,
                            &normalized_root,
                            comparisons,
                            scope,
                        )? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                })
                .map_err(|_| AuthorizerError::RootUnauthorized)?;
            if visible {
                disclosable.push(candidate);
            }
        }
        Ok(disclosable)
    }
}

fn probe_disclosable_scope(
    root_path: &str,
    normalized_root: &crate::journal_broker::wire::NormalizedWindowsPath,
    root_comparisons: &[NameComparisonSemantics],
    scope: &RootRelativeScope,
) -> Result<bool, WindowsSecurityError> {
    probe_disclosable_scope_with(
        root_path,
        scope,
        |path| {
            let handle = open_directory_handle(&path.encode_utf16().collect::<Vec<_>>())?;
            verify_handle_under_root(handle.raw(), normalized_root, root_comparisons)
        },
        |path| match open_metadata_handle(&path.encode_utf16().collect::<Vec<_>>()) {
            Ok(handle) => {
                verify_handle_under_root(handle.raw(), normalized_root, root_comparisons)?;
                Ok(CandidateProbe::Accessible)
            }
            Err(WindowsSecurityError::PathMissing) => Ok(CandidateProbe::Missing),
            Err(error) => Err(error),
        },
    )
}

fn verify_handle_under_root(
    handle: HANDLE,
    normalized_root: &crate::journal_broker::wire::NormalizedWindowsPath,
    root_comparisons: &[NameComparisonSemantics],
) -> Result<(), WindowsSecurityError> {
    let final_path = strip_extended_dos_prefix(&final_path(handle, FILE_NAME_NORMALIZED)?)?;
    let normalized =
        normalize_backend_path(&final_path).map_err(|_| WindowsSecurityError::RootRejected)?;
    if scope_under_root(&normalized, normalized_root, root_comparisons)
        .map_err(|_| WindowsSecurityError::RootRejected)?
        .is_none()
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateProbe {
    Accessible,
    Missing,
}

fn probe_disclosable_scope_with(
    root_path: &str,
    scope: &RootRelativeScope,
    mut probe_ancestor: impl FnMut(&str) -> Result<(), WindowsSecurityError>,
    mut probe_candidate: impl FnMut(&str) -> Result<CandidateProbe, WindowsSecurityError>,
) -> Result<bool, WindowsSecurityError> {
    probe_ancestor(root_path)?;
    let RootRelativeScope::Relative(relative) = scope else {
        return Ok(true);
    };
    let components: Vec<&str> = relative.split('/').collect();
    if components.is_empty() || components.iter().any(|component| component.is_empty()) {
        return Err(WindowsSecurityError::RootRejected);
    }
    let mut current = root_path.to_owned();
    for component in &components[..components.len().saturating_sub(1)] {
        current.push('\\');
        current.push_str(component);
        probe_ancestor(&current)?;
    }
    current.push('\\');
    current.push_str(
        components
            .last()
            .ok_or(WindowsSecurityError::RootRejected)?,
    );
    Ok(matches!(
        probe_candidate(&current)?,
        CandidateProbe::Accessible | CandidateProbe::Missing
    ))
}

struct ConnectionSecurity {
    token: OwnedToken,
    namespace: [u8; 32],
    client_binary_identity: [u8; 32],
    observed_process_id: u32,
    observed_session_id: u32,
    root_budget: WindowsRootRegistrationBudget,
    registrations: Mutex<HashMap<RootKey, RegisteredRoot>>,
}

impl ConnectionSecurity {
    fn reap_expired(&self, now: Instant) {
        if let Ok(mut registrations) = self.registrations.lock() {
            reap_registrations(&mut registrations, now);
        }
    }
}

struct RegisteredRoot {
    capability: RootCapability,
    pinned: Arc<PinnedRootState>,
    expires_at: Instant,
    _permit: RootRegistrationPermit,
}

fn insert_registration(
    security: &ConnectionSecurity,
    key: RootKey,
    capability: RootCapability,
    pinned: Arc<PinnedRootState>,
) -> Result<(), AuthorizerError> {
    let mut registrations = security
        .registrations
        .lock()
        .map_err(|_| AuthorizerError::RootUnauthorized)?;
    let now = Instant::now();
    reap_registrations(&mut registrations, now);
    let is_replacement = registrations.contains_key(&key);
    if !registration_capacity_available(registrations.len(), is_replacement) {
        return Err(AuthorizerError::RootUnauthorized);
    }
    let expires_at = now
        .checked_add(ROOT_CAPABILITY_LIFETIME)
        .ok_or(AuthorizerError::RootUnauthorized)?;
    let permit = if is_replacement {
        registrations
            .remove(&key)
            .map(|registration| registration._permit)
            .ok_or(AuthorizerError::RootUnauthorized)?
    } else {
        security.root_budget.try_acquire()?
    };
    registrations.insert(
        key,
        RegisteredRoot {
            capability,
            pinned,
            expires_at,
            _permit: permit,
        },
    );
    Ok(())
}

fn registration_is_current(expires_at: Instant, now: Instant) -> bool {
    now < expires_at
}

fn reap_registrations(registrations: &mut HashMap<RootKey, RegisteredRoot>, now: Instant) {
    registrations.retain(|_, registration| registration_is_current(registration.expires_at, now));
}

fn registration_capacity_available(current: usize, is_replacement: bool) -> bool {
    is_replacement || current < MAX_REGISTERED_ROOTS_PER_CONNECTION
}

#[derive(Clone)]
pub(in crate::journal_broker) struct WindowsBrokerSecurity(Arc<ConnectionSecurity>);

impl WindowsBrokerSecurity {
    pub(in crate::journal_broker) fn pinned_root(
        &self,
        root: &AuthorizedPinnedRoot,
    ) -> Result<Arc<PinnedRootState>, WindowsSecurityError> {
        if root.caller.opaque_namespace != self.0.namespace
            || root.pinned_root.volume_id != root.declared_root.volume_id
            || root.pinned_root.root_identity != root.declared_root.root_identity
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        let mut registrations = self
            .0
            .registrations
            .lock()
            .map_err(|_| WindowsSecurityError::RootRejected)?;
        reap_registrations(&mut registrations, Instant::now());
        registrations
            .get(&RootKey {
                root_id: root.declared_root.root_id.clone(),
                root_generation: root.declared_root.root_generation,
            })
            .map(|registration| Arc::clone(&registration.pinned))
            .filter(|pinned| {
                pinned.volume_id == root.declared_root.volume_id
                    && pinned.root_identity == root.declared_root.root_identity
                    && pinned.canonical_root_utf16 == root.declared_root.canonical_root_utf16
            })
            .ok_or(WindowsSecurityError::RootRejected)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RootKey {
    root_id: String,
    root_generation: u64,
}

pub(in crate::journal_broker) struct PinnedRootState {
    #[allow(
        dead_code,
        reason = "the live root handle is intentionally retained to pin identity for the request"
    )]
    root_handle: OwnedFileHandle,
    volume_handle: Option<OwnedFileHandle>,
    volume_id: String,
    volume_open_path: String,
    root_identity: Vec<u8>,
    canonical_root_utf16: Vec<u16>,
    component_semantics: Vec<NameComparisonSemantics>,
}

impl PinnedRootState {
    pub(in crate::journal_broker) fn volume_handle(&self) -> Result<HANDLE, WindowsSecurityError> {
        self.volume_handle
            .as_ref()
            .map(OwnedFileHandle::raw)
            .ok_or(WindowsSecurityError::RootRejected)
    }

    pub(in crate::journal_broker) fn canonical_root_utf16(&self) -> &[u16] {
        &self.canonical_root_utf16
    }

    pub(in crate::journal_broker) fn volume_open_path(&self) -> &str {
        &self.volume_open_path
    }

    pub(in crate::journal_broker) fn volume_id(&self) -> &str {
        &self.volume_id
    }

    pub(in crate::journal_broker) fn component_semantics(&self) -> &[NameComparisonSemantics] {
        &self.component_semantics
    }

    pub(in crate::journal_broker) fn revalidate_identity(
        &self,
    ) -> Result<(), WindowsSecurityError> {
        reject_reparse_handle(self.root_handle.raw())?;
        let final_dos =
            strip_extended_dos_prefix(&final_path(self.root_handle.raw(), FILE_NAME_NORMALIZED)?)?;
        if !root_paths_equal_by_parent_semantics(
            &final_dos,
            &self.canonical_root_utf16,
            &self.component_semantics,
        )
        .map_err(|_| WindowsSecurityError::RootRejected)?
            || file_identity(self.root_handle.raw())?.as_slice() != self.root_identity.as_slice()
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        let final_guid = final_path(
            self.root_handle.raw(),
            FILE_NAME_NORMALIZED | VOLUME_NAME_GUID,
        )?;
        let (volume_open_path, _) = split_guid_path(&final_guid)?;
        let (filesystem, _) = volume_information(self.root_handle.raw())?;
        if normalize_volume_id(&volume_open_path) != normalize_volume_id(&self.volume_id)
            || !filesystem.eq_ignore_ascii_case("NTFS")
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        Ok(())
    }
}

impl ConnectionSecurity {
    #[allow(
        dead_code,
        reason = "R2c-O root pinning uses the captured token in the next admission slice"
    )]
    fn impersonate<T>(
        &self,
        operation: impl FnOnce() -> Result<T, WindowsSecurityError>,
    ) -> Result<T, WindowsSecurityError> {
        // SAFETY: token is a live impersonation token with TOKEN_IMPERSONATE. The current thread
        // performs no callback while impersonated and the result is not returned until revert is
        // checked below.
        if unsafe { ImpersonateLoggedOnUser(self.token.raw()) } == 0 {
            return Err(WindowsSecurityError::ImpersonationFailed);
        }
        let result = operation();
        // SAFETY: this thread successfully impersonated above. A failed revert is a fatal
        // admission failure and no request result is published.
        if unsafe { RevertToSelf() } == 0 {
            return Err(WindowsSecurityError::RevertFailed);
        }
        result
    }

    fn duplicate_and_pin_client_root(
        &self,
        client_root_handle: u64,
        root: &RootAuthorization,
    ) -> Result<PinnedRootState, WindowsSecurityError> {
        let source_handle = client_root_handle as usize as HANDLE;
        if source_handle.is_null() || source_handle == INVALID_HANDLE_VALUE {
            return Err(WindowsSecurityError::RootRejected);
        }
        // SAFETY: the PID was observed from the connected named pipe, no handle is inherited, and
        // PROCESS_DUP_HANDLE is the only requested process right.
        let process = OwnedFileHandle::new(unsafe {
            OpenProcess(PROCESS_DUP_HANDLE, 0, self.observed_process_id)
        })?;
        let mut duplicated = null_mut();
        // SAFETY: source process is the server-observed pipe client, source_handle is treated only
        // as an opaque handle value in that process, the target is this broker process, and the
        // duplicated access is reduced to traverse/attributes and cannot enumerate or read content.
        if unsafe {
            DuplicateHandle(
                process.raw(),
                source_handle,
                GetCurrentProcess(),
                &mut duplicated,
                ROOT_DUPLICATED_ACCESS,
                0,
                0,
            )
        } == 0
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        pin_root_handle(root, OwnedFileHandle::new(duplicated)?)
    }
}

struct OwnedToken(HANDLE);

impl OwnedToken {
    fn new(raw: HANDLE) -> Result<Self, WindowsSecurityError> {
        if raw.is_null() {
            Err(WindowsSecurityError::TokenUnavailable)
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

// SAFETY: the token is a kernel handle with one RAII owner. Windows permits querying and using an
// impersonation token from another thread; each impersonation affects only the calling thread and
// every use is paired with a checked RevertToSelf before the operation returns.
unsafe impl Send for OwnedToken {}
// SAFETY: shared access never mutates Rust memory behind the handle and Win32 token operations are
// thread-safe. Per-thread impersonation state is not stored in OwnedToken.
unsafe impl Sync for OwnedToken {}

impl Drop for OwnedToken {
    fn drop(&mut self) {
        // SAFETY: this unique owner was constructed from a non-null token handle and closes once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct OwnedProcessHandle(HANDLE);

impl OwnedProcessHandle {
    fn new(raw: HANDLE) -> Result<Self, WindowsSecurityError> {
        if raw.is_null() {
            Err(WindowsSecurityError::TokenUnavailable)
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        // SAFETY: this unique owner was constructed from a non-null process handle and closes once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub(in crate::journal_broker) struct OwnedFileHandle(HANDLE);

impl OwnedFileHandle {
    fn new(raw: HANDLE) -> Result<Self, WindowsSecurityError> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(WindowsSecurityError::RootRejected)
        } else {
            Ok(Self(raw))
        }
    }

    pub(in crate::journal_broker) fn raw(&self) -> HANDLE {
        self.0
    }
}

// SAFETY: this is a unique kernel file-handle owner. Operations use caller-provided buffers only
// for their synchronous call and the handle value itself may be moved between worker threads.
unsafe impl Send for OwnedFileHandle {}
// SAFETY: shared access does not mutate Rust memory behind the handle. Mutating journal operations
// are not exposed, and service request serialization owns any DeviceIoControl buffer.
unsafe impl Sync for OwnedFileHandle {}

impl Drop for OwnedFileHandle {
    fn drop(&mut self) {
        // SAFETY: construction rejected null and INVALID_HANDLE_VALUE and this owner closes once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn pin_root_handle(
    root: &RootAuthorization,
    root_handle: OwnedFileHandle,
) -> Result<PinnedRootState, WindowsSecurityError> {
    let mut standard = FILE_STANDARD_INFO::default();
    // SAFETY: standard is live and exactly sized; the duplicated handle must name a directory.
    if unsafe {
        GetFileInformationByHandleEx(
            root_handle.raw(),
            FileStandardInfo,
            (&raw mut standard).cast(),
            u32::try_from(size_of::<FILE_STANDARD_INFO>())
                .map_err(|_| WindowsSecurityError::RootRejected)?,
        )
    } == 0
        || !standard.Directory
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    reject_reparse_handle(root_handle.raw())?;
    let component_semantics = query_component_semantics(&root.canonical_root_utf16)?;
    let final_dos = final_path(root_handle.raw(), FILE_NAME_NORMALIZED)?;
    let final_dos = strip_extended_dos_prefix(&final_dos)?;
    if !root_paths_equal_by_parent_semantics(
        &final_dos,
        &root.canonical_root_utf16,
        &component_semantics,
    )
    .map_err(|_| WindowsSecurityError::RootRejected)?
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    let final_guid = final_path(root_handle.raw(), FILE_NAME_NORMALIZED | VOLUME_NAME_GUID)?;
    let (volume_open_path, _) = split_guid_path(&final_guid)?;
    if normalize_volume_id(&volume_open_path) != normalize_volume_id(&root.volume_id) {
        return Err(WindowsSecurityError::RootRejected);
    }
    let (filesystem, _) = volume_information(root_handle.raw())?;
    if !filesystem.eq_ignore_ascii_case("NTFS") {
        return Err(WindowsSecurityError::UnsupportedFilesystem);
    }
    let actual_identity = file_identity(root_handle.raw())?;
    if !root_identity_matches(&root.root_identity, &actual_identity) {
        return Err(WindowsSecurityError::RootRejected);
    }
    Ok(PinnedRootState {
        root_handle,
        volume_handle: None,
        volume_id: root.volume_id.clone(),
        volume_open_path,
        root_identity: root.root_identity.clone(),
        canonical_root_utf16: root.canonical_root_utf16.clone(),
        component_semantics,
    })
}

#[cfg(any(test, feature = "broker-acceptance-client"))]
pub(in crate::journal_broker) fn describe_disposable_root_for_acceptance(
    path: &Path,
) -> Result<RootAuthorization, WindowsSecurityError> {
    let (root, _, _) = describe_client_root(path, "r2c-o-disposable-acceptance", 1)?;
    Ok(root)
}

pub(in crate::journal_broker) fn describe_client_root(
    path: &Path,
    root_id: &str,
    root_generation: u64,
) -> Result<(RootAuthorization, OwnedFileHandle, u64), WindowsSecurityError> {
    let supplied: Vec<u16> = path.as_os_str().encode_wide().collect();
    let root_handle = open_directory_handle(&supplied)?;
    let canonical_root_utf16 =
        strip_extended_dos_prefix(&final_path(root_handle.raw(), FILE_NAME_NORMALIZED)?)?;
    let final_guid = final_path(root_handle.raw(), FILE_NAME_NORMALIZED | VOLUME_NAME_GUID)?;
    let (volume_id, _) = split_guid_path(&final_guid)?;
    let (filesystem, volume_serial) = volume_information(root_handle.raw())?;
    if !filesystem.eq_ignore_ascii_case("NTFS") {
        return Err(WindowsSecurityError::UnsupportedFilesystem);
    }
    let root = RootAuthorization {
        root_id: root_id.to_owned(),
        root_generation,
        volume_id,
        root_identity: file_identity(root_handle.raw())?.to_vec(),
        canonical_root_utf16,
    };
    root.validate()
        .map_err(|_| WindowsSecurityError::RootRejected)?;
    Ok((root, root_handle, u64::from(volume_serial)))
}

fn root_identity_matches(declared: &[u8], actual: &[u8; 16]) -> bool {
    declared == actual
}

pub(in crate::journal_broker) fn open_directory_handle(
    path: &[u16],
) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let path = nul_terminated_path(path)?;
    // SAFETY: path is bounded, NUL-terminated, and live. Access is metadata/traverse only; share
    // flags avoid changing source state and the checked handle has one owner.
    let raw = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    let handle = OwnedFileHandle::new(raw)?;
    reject_reparse_handle(handle.raw())?;
    Ok(handle)
}

fn open_metadata_handle(path: &[u16]) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let path = nul_terminated_path(path)?;
    // SAFETY: path is bounded, terminated, and live. FILE_READ_ATTRIBUTES cannot read file
    // content or mutate the object; BACKUP_SEMANTICS permits the same check for directories.
    let raw = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if raw.is_null() || raw == INVALID_HANDLE_VALUE {
        return match io::Error::last_os_error()
            .raw_os_error()
            .map(|value| value as u32)
        {
            Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) => {
                Err(WindowsSecurityError::PathMissing)
            }
            _ => Err(WindowsSecurityError::RootRejected),
        };
    }
    let handle = OwnedFileHandle::new(raw)?;
    reject_reparse_handle(handle.raw())?;
    Ok(handle)
}

fn reject_reparse_handle(handle: HANDLE) -> Result<(), WindowsSecurityError> {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: attributes is live and exactly sized for FileAttributeTagInfo; the handle is live.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&raw mut attributes).cast(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .map_err(|_| WindowsSecurityError::RootRejected)?,
        )
    } == 0
        || is_reparse_attributes(attributes.FileAttributes)
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    Ok(())
}

fn is_reparse_attributes(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn open_volume_handle(path: &str) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let trimmed = path.trim_end_matches('\\');
    let mut path: Vec<u16> = trimmed.encode_utf16().collect();
    if path.is_empty() || path.len() >= MAX_WIN32_PATH_UNITS || path.contains(&0) {
        return Err(WindowsSecurityError::RootRejected);
    }
    path.push(0);
    // SAFETY: the verified volume GUID path is NUL-terminated and live. GENERIC_READ is required
    // for existing-journal queries; no create/delete journal control is present in this process.
    let raw = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            VOLUME_OPEN_FLAGS,
            null_mut(),
        )
    };
    OwnedFileHandle::new(raw)
}

fn nul_terminated_path(path: &[u16]) -> Result<Vec<u16>, WindowsSecurityError> {
    if path.is_empty() || path.len() >= MAX_WIN32_PATH_UNITS || path.contains(&0) {
        return Err(WindowsSecurityError::RootRejected);
    }
    let mut terminated = path.to_vec();
    terminated.push(0);
    Ok(terminated)
}

fn final_path(handle: HANDLE, flags: u32) -> Result<Vec<u16>, WindowsSecurityError> {
    let mut capacity = 512_usize;
    loop {
        if capacity > MAX_WIN32_PATH_UNITS {
            return Err(WindowsSecurityError::RootRejected);
        }
        let mut buffer = vec![0_u16; capacity];
        // SAFETY: buffer is initialized, live, and its checked capacity is provided to Win32. The
        // returned length is validated before any UTF-16 unit is used.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).map_err(|_| WindowsSecurityError::RootRejected)?,
                flags,
            )
        } as usize;
        if length == 0 {
            return Err(WindowsSecurityError::RootRejected);
        }
        if length < buffer.len() {
            buffer.truncate(length);
            if String::from_utf16(&buffer).is_err() {
                return Err(WindowsSecurityError::RootRejected);
            }
            return Ok(buffer);
        }
        capacity = length
            .checked_add(1)
            .ok_or(WindowsSecurityError::RootRejected)?;
    }
}

fn strip_extended_dos_prefix(path: &[u16]) -> Result<Vec<u16>, WindowsSecurityError> {
    let prefix: Vec<u16> = r"\\?\".encode_utf16().collect();
    path.strip_prefix(prefix.as_slice())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(WindowsSecurityError::RootRejected)
}

fn split_guid_path(path: &[u16]) -> Result<(String, Vec<u16>), WindowsSecurityError> {
    let decoded = String::from_utf16(path).map_err(|_| WindowsSecurityError::RootRejected)?;
    let prefix = r"\\?\Volume{";
    if !decoded.starts_with(prefix) {
        return Err(WindowsSecurityError::RootRejected);
    }
    let close = decoded
        .find("}\\")
        .ok_or(WindowsSecurityError::RootRejected)?;
    let volume_end = close
        .checked_add(2)
        .ok_or(WindowsSecurityError::RootRejected)?;
    let volume = decoded
        .get(..volume_end)
        .ok_or(WindowsSecurityError::RootRejected)?
        .to_owned();
    Ok((volume, path.to_vec()))
}

fn normalize_volume_id(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn volume_information(handle: HANDLE) -> Result<(String, u32), WindowsSecurityError> {
    let mut filesystem = vec![0_u16; 64];
    let mut serial = 0_u32;
    // SAFETY: output buffers and serial are live and bounded; optional outputs are null and Win32
    // retains no pointers.
    if unsafe {
        GetVolumeInformationByHandleW(
            handle,
            null_mut(),
            0,
            &mut serial,
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            u32::try_from(filesystem.len()).map_err(|_| WindowsSecurityError::RootRejected)?,
        )
    } == 0
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    let end = filesystem
        .iter()
        .position(|unit| *unit == 0)
        .ok_or(WindowsSecurityError::RootRejected)?;
    filesystem.truncate(end);
    let filesystem =
        String::from_utf16(&filesystem).map_err(|_| WindowsSecurityError::RootRejected)?;
    Ok((filesystem, serial))
}

fn file_identity(handle: HANDLE) -> Result<[u8; 16], WindowsSecurityError> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: info is initialized, live, and exactly sized for FileIdInfo; no pointer escapes.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_ID_INFO>())
                .map_err(|_| WindowsSecurityError::RootRejected)?,
        )
    } == 0
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    Ok(info.FileId.Identifier)
}

fn query_component_semantics(
    canonical_root: &[u16],
) -> Result<Vec<NameComparisonSemantics>, WindowsSecurityError> {
    let root =
        String::from_utf16(canonical_root).map_err(|_| WindowsSecurityError::RootRejected)?;
    let remainder = root.get(3..).ok_or(WindowsSecurityError::RootRejected)?;
    if remainder.is_empty() {
        return Ok(Vec::new());
    }
    let mut current = root
        .get(..3)
        .ok_or(WindowsSecurityError::RootRejected)?
        .to_owned();
    let components = remainder.split('\\').collect::<Vec<_>>();
    let mut semantics = Vec::with_capacity(components.len());
    for (index, component) in components.iter().enumerate() {
        if component.is_empty() {
            return Err(WindowsSecurityError::RootRejected);
        }
        let parent = open_directory_handle(&current.encode_utf16().collect::<Vec<_>>())?;
        let mut info = FILE_CASE_SENSITIVE_INFO::default();
        // SAFETY: info is live and exactly sized. The component comparison rule belongs to its
        // parent directory, including the volume root for the first component.
        if unsafe {
            GetFileInformationByHandleEx(
                parent.raw(),
                FileCaseSensitiveInfo,
                (&raw mut info).cast(),
                u32::try_from(size_of::<FILE_CASE_SENSITIVE_INFO>())
                    .map_err(|_| WindowsSecurityError::RootRejected)?,
            )
        } == 0
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        semantics.push(if info.Flags & FILE_CS_FLAG_CASE_SENSITIVE_DIR != 0 {
            NameComparisonSemantics::OrdinalCaseSensitive
        } else {
            NameComparisonSemantics::WindowsOrdinalIgnoreCase
        });
        if !current.ends_with('\\') {
            current.push('\\');
        }
        current.push_str(component);
        if index + 1 < components.len() {
            drop(open_directory_handle(
                &current.encode_utf16().collect::<Vec<_>>(),
            )?);
        }
    }
    Ok(semantics)
}

fn root_paths_equal_by_parent_semantics(
    actual: &[u16],
    declared: &[u16],
    semantics: &[NameComparisonSemantics],
) -> Result<bool, crate::journal_broker::wire::WireError> {
    let actual = normalize_backend_path(actual)?;
    let declared = normalize_backend_path(declared)?;
    if actual.component_count() != declared.component_count() {
        return Ok(false);
    }
    Ok(matches!(
        scope_under_root(&actual, &declared, semantics)?,
        Some(RootRelativeScope::Root)
    ))
}

struct TokenEvidence {
    user_sid: Vec<u8>,
    session_id: u32,
    authentication_id: LUID,
    modified_id: LUID,
    token_type: TOKEN_TYPE,
    impersonation_level: i32,
    is_elevated: bool,
}

struct PrimaryTokenEvidence {
    user_sid: Vec<u8>,
    session_id: u32,
    authentication_id: LUID,
    is_elevated: bool,
}

impl PrimaryTokenEvidence {
    fn matches_impersonation(&self, evidence: &TokenEvidence) -> bool {
        self.user_sid == evidence.user_sid
            && self.session_id == evidence.session_id
            && self.authentication_id.LowPart == evidence.authentication_id.LowPart
            && self.authentication_id.HighPart == evidence.authentication_id.HighPart
            && self.is_elevated == evidence.is_elevated
    }
}

fn capture_named_pipe_client_token(pipe: HANDLE) -> Result<OwnedToken, WindowsSecurityError> {
    // SAFETY: pipe is a connected server-side named pipe. Impersonation applies only to the
    // current host thread and is reverted on every path after this succeeds.
    if unsafe { ImpersonateNamedPipeClient(pipe) } == 0 {
        return Err(WindowsSecurityError::ImpersonationFailed);
    }
    let result = (|| {
        let mut thread_token = null_mut();
        // SAFETY: GetCurrentThread returns a pseudo-handle valid for this call; output is a live
        // handle slot and requested rights are limited to query/duplicate/impersonate.
        if unsafe {
            OpenThreadToken(
                GetCurrentThread(),
                TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_IMPERSONATE,
                0,
                &mut thread_token,
            )
        } == 0
        {
            return Err(WindowsSecurityError::TokenUnavailable);
        }
        let thread_token = OwnedToken::new(thread_token)?;
        let mut duplicated = null_mut();
        // SAFETY: source is a live impersonation token; null attributes are allowed; output is a
        // live handle slot. The duplicate requests only the rights used by this broker.
        if unsafe {
            DuplicateTokenEx(
                thread_token.raw(),
                TOKEN_QUERY | TOKEN_IMPERSONATE,
                null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut duplicated,
            )
        } == 0
        {
            return Err(WindowsSecurityError::TokenUnavailable);
        }
        OwnedToken::new(duplicated)
    })();
    // SAFETY: the current thread was successfully impersonated above. This call occurs for both
    // success and error results before either can leave this boundary.
    if unsafe { RevertToSelf() } == 0 {
        return Err(WindowsSecurityError::RevertFailed);
    }
    result
}

fn query_primary_token_evidence(
    process_id: u32,
) -> Result<PrimaryTokenEvidence, WindowsSecurityError> {
    // SAFETY: the process id came from GetNamedPipeClientProcessId and access is query-only.
    let process = OwnedProcessHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id)
    })?;
    let mut raw_token = null_mut();
    // SAFETY: process is a live query handle and raw_token is a live output slot.
    if unsafe { OpenProcessToken(process.raw(), TOKEN_QUERY, &mut raw_token) } == 0 {
        return Err(WindowsSecurityError::TokenUnavailable);
    }
    let token = OwnedToken::new(raw_token)?;
    let token_type = read_token_scalar::<TOKEN_TYPE>(&token, TokenType)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary token type"))?;
    if token_type != TokenPrimary {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "primary token type value",
        ));
    }
    let session_id = read_token_scalar::<u32>(&token, TokenSessionId)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary session"))?;
    let statistics = query_token_information(&token, TokenStatistics)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary statistics"))?;
    if statistics.len() < size_of::<TOKEN_STATISTICS>() {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "primary statistics size",
        ));
    }
    // SAFETY: size was checked and read_unaligned avoids assuming Vec<u8> alignment.
    let statistics =
        unsafe { std::ptr::read_unaligned(statistics.as_ptr().cast::<TOKEN_STATISTICS>()) };
    let user_sid = query_token_sid_bytes(&token, "primary user")?;
    let elevation = read_token_scalar::<TOKEN_ELEVATION>(&token, TokenElevation)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary elevation"))?;
    Ok(PrimaryTokenEvidence {
        user_sid,
        session_id,
        authentication_id: statistics.AuthenticationId,
        is_elevated: elevation.TokenIsElevated != 0,
    })
}

fn query_token_evidence(
    token: &OwnedToken,
    primary_is_elevated: bool,
) -> Result<TokenEvidence, WindowsSecurityError> {
    let session = query_token_information(token, TokenSessionId)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("session query"))?;
    let session_bytes: [u8; 4] = session
        .get(..4)
        .ok_or(WindowsSecurityError::TokenEvidenceRejected("session size"))?
        .try_into()
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("session bytes"))?;
    let statistics = query_token_information(token, TokenStatistics)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("statistics query"))?;
    if statistics.len() < size_of::<TOKEN_STATISTICS>() {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "statistics size",
        ));
    }
    // SAFETY: size was checked and read_unaligned avoids assuming Vec<u8> alignment. The copied
    // structure contains only scalar token statistics used below.
    let statistics =
        unsafe { std::ptr::read_unaligned(statistics.as_ptr().cast::<TOKEN_STATISTICS>()) };
    let user_sid = query_token_sid_bytes(token, "impersonation user")?;
    let token_type = read_token_scalar::<TOKEN_TYPE>(token, TokenType)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("token type"))?;
    let impersonation_level = read_token_scalar::<i32>(token, TokenImpersonationLevel)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("impersonation level"))?;
    if token_type != TokenImpersonation || impersonation_level < SecurityImpersonation {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "token type or impersonation level",
        ));
    }
    Ok(TokenEvidence {
        user_sid,
        session_id: u32::from_ne_bytes(session_bytes),
        authentication_id: statistics.AuthenticationId,
        modified_id: statistics.ModifiedId,
        token_type,
        impersonation_level,
        is_elevated: primary_is_elevated,
    })
}

fn query_token_sid_bytes(
    token: &OwnedToken,
    stage: &'static str,
) -> Result<Vec<u8>, WindowsSecurityError> {
    let user = query_token_information(token, TokenUser)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected(stage))?;
    if user.len() < size_of::<TOKEN_USER>() {
        return Err(WindowsSecurityError::TokenEvidenceRejected(stage));
    }
    // SAFETY: size was checked and read_unaligned avoids assuming Vec<u8> alignment. The SID
    // pointer is range-checked against the same live output buffer before copying.
    let user_header = unsafe { std::ptr::read_unaligned(user.as_ptr().cast::<TOKEN_USER>()) };
    let sid = user_header.User.Sid;
    // SAFETY: the pointer originated in a successful TokenUser result and remains live with user.
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(WindowsSecurityError::TokenEvidenceRejected(stage));
    }
    // SAFETY: IsValidSid succeeded for this live token-information buffer.
    let sid_length = unsafe { GetLengthSid(sid) } as usize;
    if sid_length == 0 || sid_length > MAX_SID_BYTES {
        return Err(WindowsSecurityError::TokenEvidenceRejected(stage));
    }
    let buffer_start = user.as_ptr() as usize;
    let buffer_end = buffer_start
        .checked_add(user.len())
        .ok_or(WindowsSecurityError::TokenMalformed)?;
    let sid_start = sid as usize;
    let sid_end = sid_start
        .checked_add(sid_length)
        .ok_or(WindowsSecurityError::TokenMalformed)?;
    if sid_start < buffer_start || sid_end > buffer_end {
        return Err(WindowsSecurityError::TokenEvidenceRejected(stage));
    }
    // SAFETY: the SID byte range was proven to lie wholly inside the live user buffer.
    Ok(unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), sid_length) }.to_vec())
}

fn read_token_scalar<T: Copy>(
    token: &OwnedToken,
    information_class: i32,
) -> Result<T, WindowsSecurityError> {
    let bytes = query_token_information(token, information_class)?;
    if bytes.len() != size_of::<T>() {
        return Err(WindowsSecurityError::TokenMalformed);
    }
    // SAFETY: the information class returned exactly one initialized T and read_unaligned avoids
    // imposing Vec<u8> alignment.
    Ok(unsafe { bytes.as_ptr().cast::<T>().read_unaligned() })
}

fn query_token_information(
    token: &OwnedToken,
    class: i32,
) -> Result<Vec<u8>, WindowsSecurityError> {
    let mut required = 0_u32;
    // SAFETY: null buffer with zero length is the documented size query; required is live.
    let _ = unsafe { GetTokenInformation(token.raw(), class, null_mut(), 0, &mut required) };
    if required == 0 || required as usize > MAX_TOKEN_INFORMATION_BYTES {
        return Err(WindowsSecurityError::TokenMalformed);
    }
    let mut buffer = vec![0_u8; required as usize];
    let mut returned = required;
    // SAFETY: buffer is initialized, bounded, live, and sized exactly from the first query; no
    // pointer escapes and the returned size is checked before use.
    if unsafe {
        GetTokenInformation(
            token.raw(),
            class,
            buffer.as_mut_ptr().cast(),
            required,
            &mut returned,
        )
    } == 0
        || returned == 0
        || returned > required
    {
        return Err(WindowsSecurityError::TokenMalformed);
    }
    buffer.truncate(returned as usize);
    Ok(buffer)
}

fn token_binding(
    evidence: &TokenEvidence,
    connection_id: [u8; 16],
    connection_generation: u64,
    connection_nonce: u64,
    process_id: u32,
    session_id: u32,
    client_binary_identity: [u8; 32],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-pipe-token-binding-v1\0");
    hasher.update(&connection_id);
    hasher.update(&connection_generation.to_le_bytes());
    hasher.update(&connection_nonce.to_le_bytes());
    hasher.update(&process_id.to_le_bytes());
    hasher.update(&session_id.to_le_bytes());
    hasher.update(&client_binary_identity);
    hasher.update(&evidence.user_sid);
    hasher.update(&evidence.authentication_id.LowPart.to_le_bytes());
    hasher.update(&evidence.authentication_id.HighPart.to_le_bytes());
    hasher.update(&evidence.modified_id.LowPart.to_le_bytes());
    hasher.update(&evidence.modified_id.HighPart.to_le_bytes());
    hasher.update(&evidence.token_type.to_le_bytes());
    hasher.update(&evidence.impersonation_level.to_le_bytes());
    hasher.update(&[u8::from(evidence.is_elevated)]);
    *hasher.finalize().as_bytes()
}

fn disposable_client_identity(
    connection_id: [u8; 16],
    connection_generation: u64,
    connection_nonce: u64,
    process_id: u32,
    session_id: u32,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-disposable-client-identity-v1\0");
    hasher.update(&connection_id);
    hasher.update(&connection_generation.to_le_bytes());
    hasher.update(&connection_nonce.to_le_bytes());
    hasher.update(&process_id.to_le_bytes());
    hasher.update(&session_id.to_le_bytes());
    *hasher.finalize().as_bytes()
}

fn luid_bytes(value: LUID) -> [u8; 8] {
    let mut bytes = [0_u8; 8];
    bytes[..4].copy_from_slice(&value.LowPart.to_le_bytes());
    bytes[4..].copy_from_slice(&value.HighPart.to_le_bytes());
    bytes
}

#[derive(Debug, thiserror::Error)]
pub(in crate::journal_broker) enum WindowsSecurityError {
    #[error("named-pipe caller was rejected")]
    CallerRejected,
    #[error("named-pipe caller impersonation failed")]
    ImpersonationFailed,
    #[error("named-pipe caller token was unavailable")]
    TokenUnavailable,
    #[error("named-pipe caller token evidence was malformed")]
    TokenMalformed,
    #[error("named-pipe caller token evidence rejected at {0}")]
    TokenEvidenceRejected(&'static str),
    #[error("named-pipe caller impersonation could not be reverted")]
    RevertFailed,
    #[error("declared root identity or containment was rejected")]
    RootRejected,
    #[error("declared root filesystem is unsupported")]
    UnsupportedFilesystem,
    #[error("candidate path no longer exists")]
    PathMissing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_authorizer_rejects_client_claims_without_os_token() {
        let authenticated = AuthenticatedConnection::from_pipe_token(
            [1; 16],
            1,
            AuthenticatedPipeFacts {
                opaque_token_binding: [2; 32],
                client_binary_identity: [3; 32],
                process_id: 100,
                session_id: 3,
                token_type: 2,
                impersonation_level: 2,
                is_elevated: false,
            },
        )
        .expect("authenticated connection");
        let claim = CallerClaim {
            process_id: 100,
            session_id: 3,
            client_instance: [4; 16],
        };
        assert!(matches!(
            WindowsPipeAuthorizer::fail_closed_for_test().verify_claim(&authenticated, &claim),
            Err(AuthorizerError::CallerRejected)
        ));
    }

    #[test]
    fn root_identity_requires_the_complete_file_id() {
        let actual = [7_u8; 16];
        assert!(root_identity_matches(&actual, &actual));
        assert!(!root_identity_matches(&actual[..8], &actual));
    }

    #[test]
    fn capability_lease_and_per_connection_capacity_are_strictly_bounded() {
        let now = Instant::now();
        assert!(registration_is_current(now + Duration::from_millis(1), now));
        assert!(!registration_is_current(now, now));
        assert!(registration_capacity_available(7, false));
        assert!(!registration_capacity_available(8, false));
        assert!(registration_capacity_available(8, true));
    }

    #[test]
    fn global_root_budget_is_shared_and_reclaimed() {
        let budget = WindowsRootRegistrationBudget::with_limit(2);
        let first = budget.try_acquire().expect("first permit");
        let second = budget.try_acquire().expect("second permit");
        assert!(matches!(
            budget.try_acquire(),
            Err(AuthorizerError::RootUnauthorized)
        ));
        drop(first);
        assert!(budget.try_acquire().is_ok());
        drop(second);
    }

    #[test]
    fn volume_handle_requires_overlapped_io() {
        assert_eq!(
            VOLUME_OPEN_FLAGS & FILE_FLAG_OVERLAPPED,
            FILE_FLAG_OVERLAPPED
        );
    }

    #[test]
    fn root_duplicate_access_cannot_enumerate_or_mutate_content() {
        assert_eq!(ROOT_DUPLICATED_ACCESS & 0x0000_0001, 0);
        assert_eq!(ROOT_DUPLICATED_ACCESS, FILE_READ_ATTRIBUTES);
    }

    #[test]
    fn parent_directory_semantics_distinguish_mixed_case_siblings() {
        let declared = r"C:\Parent\Photos".encode_utf16().collect::<Vec<_>>();
        let differently_cased = r"c:\PARENT\photos".encode_utf16().collect::<Vec<_>>();
        assert!(
            !root_paths_equal_by_parent_semantics(
                &differently_cased,
                &declared,
                &[
                    NameComparisonSemantics::WindowsOrdinalIgnoreCase,
                    NameComparisonSemantics::OrdinalCaseSensitive,
                ],
            )
            .expect("mixed semantics")
        );
        assert!(
            root_paths_equal_by_parent_semantics(
                &differently_cased,
                &declared,
                &[
                    NameComparisonSemantics::WindowsOrdinalIgnoreCase,
                    NameComparisonSemantics::WindowsOrdinalIgnoreCase,
                ],
            )
            .expect("case-insensitive semantics")
        );
    }

    #[test]
    fn reparse_attributes_are_always_rejected() {
        assert!(is_reparse_attributes(FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(!is_reparse_attributes(0));
    }

    #[test]
    fn authenticated_connection_rejects_primary_or_identification_level_pipe_tokens() {
        assert!(
            AuthenticatedConnection::from_pipe_token(
                [1; 16],
                1,
                AuthenticatedPipeFacts {
                    opaque_token_binding: [2; 32],
                    client_binary_identity: [3; 32],
                    process_id: 100,
                    session_id: 3,
                    token_type: 1,
                    impersonation_level: 2,
                    is_elevated: false,
                },
            )
            .is_err()
        );
        assert!(
            AuthenticatedConnection::from_pipe_token(
                [1; 16],
                1,
                AuthenticatedPipeFacts {
                    opaque_token_binding: [2; 32],
                    client_binary_identity: [3; 32],
                    process_id: 100,
                    session_id: 3,
                    token_type: 2,
                    impersonation_level: 1,
                    is_elevated: false,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn connection_namespace_and_disposable_identity_bind_client_binary_and_generation() {
        let evidence = TokenEvidence {
            user_sid: vec![1, 2, 3],
            session_id: 4,
            authentication_id: LUID {
                LowPart: 5,
                HighPart: 6,
            },
            modified_id: LUID {
                LowPart: 7,
                HighPart: 8,
            },
            token_type: TokenImpersonation,
            impersonation_level: SecurityImpersonation,
            is_elevated: false,
        };
        let first = token_binding(&evidence, [9; 16], 10, 11, 12, 4, [13; 32]);
        let second = token_binding(&evidence, [9; 16], 10, 11, 12, 4, [14; 32]);
        assert_ne!(first, second);
        assert_ne!(
            disposable_client_identity([9; 16], 10, 11, 12, 4),
            disposable_client_identity([9; 16], 15, 11, 12, 4)
        );
    }

    #[test]
    fn disclosure_probe_requires_every_ancestor_and_never_opens_content() {
        let mut ancestors = Vec::new();
        let mut candidates = Vec::new();
        let visible = probe_disclosable_scope_with(
            r"C:\Root",
            &RootRelativeScope::Relative("album/item.jpg".to_owned()),
            |path| {
                ancestors.push(path.to_owned());
                Ok(())
            },
            |path| {
                candidates.push(path.to_owned());
                Ok(CandidateProbe::Accessible)
            },
        )
        .expect("accessible candidate");
        assert!(visible);
        assert_eq!(ancestors, [r"C:\Root", r"C:\Root\album"]);
        assert_eq!(candidates, [r"C:\Root\album\item.jpg"]);

        assert!(
            probe_disclosable_scope_with(
                r"C:\Root",
                &RootRelativeScope::Relative("deleted.jpg".to_owned()),
                |_| Ok(()),
                |_| Ok(CandidateProbe::Missing),
            )
            .expect("enumerable parent proves deleted name disclosure")
        );
        assert!(
            probe_disclosable_scope_with(
                r"C:\Root",
                &RootRelativeScope::Relative("private/item.jpg".to_owned()),
                |path| {
                    if path.ends_with("private") {
                        Err(WindowsSecurityError::RootRejected)
                    } else {
                        Ok(())
                    }
                },
                |_| Ok(CandidateProbe::Accessible),
            )
            .is_err()
        );
        assert!(
            probe_disclosable_scope_with(
                r"C:\Root",
                &RootRelativeScope::Relative("denied.jpg".to_owned()),
                |_| Ok(()),
                |_| Err(WindowsSecurityError::RootRejected),
            )
            .is_err()
        );
    }

    #[test]
    fn disposable_acceptance_root_uses_live_volume_and_file_identity() {
        let directory = tempfile::tempdir().expect("disposable NTFS root");
        let root = describe_disposable_root_for_acceptance(directory.path())
            .expect("describe disposable acceptance root");
        assert_eq!(root.root_identity.len(), 16);
        assert!(root.volume_id.starts_with(r"\\?\Volume{"));
        assert!(root.volume_id.ends_with("}\\"));
        assert!(!root.canonical_root_utf16.is_empty());
    }

    #[test]
    fn pinned_root_revalidation_rejects_rename_and_old_path_replacement() {
        let directory = tempfile::tempdir().expect("disposable NTFS container");
        let original = directory.path().join("root");
        let moved = directory.path().join("moved-root");
        std::fs::create_dir(&original).expect("create root");
        let root = describe_disposable_root_for_acceptance(&original).expect("describe root");
        let supplied = original.as_os_str().encode_wide().collect::<Vec<_>>();
        let handle = open_directory_handle(&supplied).expect("open pinned root");
        let final_guid = final_path(handle.raw(), FILE_NAME_NORMALIZED | VOLUME_NAME_GUID)
            .expect("live GUID path");
        let (volume_open_path, _) = split_guid_path(&final_guid).expect("split GUID path");
        let component_count = normalize_backend_path(&root.canonical_root_utf16)
            .expect("normalize root")
            .component_count();
        let pinned = PinnedRootState {
            root_handle: handle,
            volume_handle: None,
            volume_id: root.volume_id.clone(),
            volume_open_path,
            root_identity: root.root_identity.clone(),
            canonical_root_utf16: root.canonical_root_utf16.clone(),
            component_semantics: vec![
                NameComparisonSemantics::WindowsOrdinalIgnoreCase;
                component_count
            ],
        };
        std::fs::rename(&original, &moved).expect("rename pinned root");
        std::fs::create_dir(&original).expect("replace old path");

        let result = pinned.revalidate_identity();
        drop(pinned);
        std::fs::remove_dir(&original).expect("remove replacement");
        std::fs::rename(&moved, &original).expect("restore temp root");
        assert!(matches!(result, Err(WindowsSecurityError::RootRejected)));
    }

    #[test]
    fn idle_maintenance_reaps_expired_root_and_releases_global_permit() {
        let directory = tempfile::tempdir().expect("disposable NTFS root");
        let root =
            describe_disposable_root_for_acceptance(directory.path()).expect("describe root");
        let supplied = directory
            .path()
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>();
        let handle = open_directory_handle(&supplied).expect("open pinned root");
        let final_guid = final_path(handle.raw(), FILE_NAME_NORMALIZED | VOLUME_NAME_GUID)
            .expect("live GUID path");
        let (volume_open_path, _) = split_guid_path(&final_guid).expect("split GUID path");
        let component_count = normalize_backend_path(&root.canonical_root_utf16)
            .expect("normalize root")
            .component_count();
        let pinned = Arc::new(PinnedRootState {
            root_handle: handle,
            volume_handle: None,
            volume_id: root.volume_id,
            volume_open_path,
            root_identity: root.root_identity,
            canonical_root_utf16: root.canonical_root_utf16,
            component_semantics: vec![
                NameComparisonSemantics::WindowsOrdinalIgnoreCase;
                component_count
            ],
        });
        let budget = WindowsRootRegistrationBudget::with_limit(1);
        let permit = budget.try_acquire().expect("global root permit");
        let now = Instant::now();
        let mut registrations = HashMap::from([(
            RootKey {
                root_id: "root-expired".to_owned(),
                root_generation: 1,
            },
            RegisteredRoot {
                capability: RootCapability([9; 32]),
                pinned,
                expires_at: now,
                _permit: permit,
            },
        )]);
        assert_eq!(budget.active_count(), 1);

        reap_registrations(&mut registrations, now);

        assert!(registrations.is_empty());
        assert_eq!(budget.active_count(), 0);
        assert!(budget.try_acquire().is_ok());
    }
}
