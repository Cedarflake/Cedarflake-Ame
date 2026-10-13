use windows_sys::Win32::Security::TokenPrimary;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::process_token::{ProcessToken, ProcessTokenIdentity};
use super::transport::WindowsTransportError;

pub(super) const SERVICE_SID: &str =
    "S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874";
pub(super) const LOCAL_SYSTEM_SID: &str = "S-1-5-18";
const VOLUME_PRIVILEGE: &str = "SeManageVolumePrivilege";
const IMPERSONATION_PRIVILEGE: &str = "SeImpersonatePrivilege";
const TRAVERSE_PRIVILEGE: &str = "SeChangeNotifyPrivilege";

pub(super) fn verify_running_service_profile() -> Result<(), WindowsTransportError> {
    // SAFETY: the pseudo-handle names this process and is borrowed only for the token query.
    let token = ProcessToken::open(unsafe { GetCurrentProcess() })?;
    validate_service_profile(&token.identity()?, &token.privilege_names()?)
}

fn validate_service_profile(
    identity: &ProcessTokenIdentity,
    privileges: &[String],
) -> Result<(), WindowsTransportError> {
    if identity.user_sid != LOCAL_SYSTEM_SID
        || identity.token_type != TokenPrimary
        || !identity.is_elevated
        || !identity.is_restricted
        || !identity
            .restricted_sids
            .iter()
            .any(|sid| sid == SERVICE_SID)
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }

    // SCM retains SeChangeNotifyPrivilege even when RequiredPrivileges omits it. Every other
    // privilege must be removed, including disabled privileges that could later be enabled.
    let mut volume_privileges = 0;
    let mut impersonation_privileges = 0;
    let mut traverse_privileges = 0;
    for privilege in privileges {
        match privilege.as_str() {
            VOLUME_PRIVILEGE => volume_privileges += 1,
            IMPERSONATION_PRIVILEGE => impersonation_privileges += 1,
            TRAVERSE_PRIVILEGE => traverse_privileges += 1,
            _ => return Err(WindowsTransportError::ServerIdentityRejected),
        }
    }
    if volume_privileges != 1 || impersonation_privileges != 1 || traverse_privileges > 1 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
