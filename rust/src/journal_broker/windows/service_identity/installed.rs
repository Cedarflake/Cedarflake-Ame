use std::path::PathBuf;

use windows_sys::Win32::Security::{TOKEN_TYPE, TokenPrimary};

use super::WindowsTransportError;
use crate::journal_broker::windows::service_profile::{LOCAL_SYSTEM_SID, SERVICE_SID};
use crate::journal_broker::{MAX_FRAME_BYTES, PROTOCOL_VERSION};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InstalledBrokerIdentity {
    pub(super) protocol: u16,
    pub(super) max_frame: usize,
    pub(super) binary_sha256: [u8; 32],
}

impl InstalledBrokerIdentity {
    pub(super) fn parse(value: &str) -> Result<Self, WindowsTransportError> {
        let fields: Vec<&str> = value.split('|').collect();
        let ["AMEJBID3", protocol, max_frame, digest] = fields.as_slice() else {
            return Err(WindowsTransportError::ServerIdentityRejected);
        };
        if *protocol != format!("protocol={PROTOCOL_VERSION}")
            || *max_frame != format!("max_frame={MAX_FRAME_BYTES}")
        {
            return Err(WindowsTransportError::ServerIdentityRejected);
        }
        let digest = digest
            .strip_prefix("sha256=")
            .filter(|value| value.len() == 64)
            .ok_or(WindowsTransportError::ServerIdentityRejected)?;
        let mut binary_sha256 = [0; 32];
        for (slot, pair) in binary_sha256
            .iter_mut()
            .zip(digest.as_bytes().chunks_exact(2))
        {
            *slot = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
        }
        Ok(Self {
            protocol: PROTOCOL_VERSION,
            max_frame: MAX_FRAME_BYTES,
            binary_sha256,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ServerProcessFacts {
    pub(super) scm_process_id: u32,
    pub(super) pipe_process_id: u32,
    pub(super) image_path: PathBuf,
    pub(super) expected_image_path: PathBuf,
    pub(super) token_user_sid: String,
    pub(super) token_type: TOKEN_TYPE,
    pub(super) is_elevated: bool,
    pub(super) is_restricted: bool,
    pub(super) restricted_sids: Vec<String>,
    pub(super) manifest: InstalledBrokerIdentity,
    pub(super) actual_binary_sha256: [u8; 32],
}

impl ServerProcessFacts {
    pub(super) fn validate(&self) -> Result<(), WindowsTransportError> {
        if self.scm_process_id == 0
            || self.pipe_process_id != self.scm_process_id
            || !self
                .image_path
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&self.expected_image_path.as_os_str().to_string_lossy())
            || self.token_user_sid != LOCAL_SYSTEM_SID
            || self.token_type != TokenPrimary
            || !self.is_elevated
            || !self.is_restricted
            || !self.restricted_sids.iter().any(|sid| sid == SERVICE_SID)
            || self.manifest.protocol != PROTOCOL_VERSION
            || self.manifest.max_frame != MAX_FRAME_BYTES
            || self.manifest.binary_sha256 != self.actual_binary_sha256
        {
            return Err(WindowsTransportError::ServerIdentityRejected);
        }
        Ok(())
    }
}

fn hex_nibble(value: u8) -> Result<u8, WindowsTransportError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(WindowsTransportError::ServerIdentityRejected),
    }
}

#[cfg(test)]
mod tests;
