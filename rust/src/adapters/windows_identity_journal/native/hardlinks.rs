use std::collections::BTreeSet;
use std::io;

use windows_sys::Win32::Foundation::{
    ERROR_HANDLE_EOF, ERROR_NOT_SUPPORTED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{FindClose, FindFirstFileNameW, FindNextFileNameW};

use crate::domain::{FileIdentityEvidence, ScanError};

use super::{MAX_PATH_UNITS, invalid, metadata_error, terminated};

// NTFS permits the original directory entry plus 1,023 additional hard links.
const MAX_LINKS: usize = 1_024;

pub(super) struct HardlinkLookup {
    pub(super) identity: FileIdentityEvidence,
    pub(super) link_count: u32,
    pub(super) attributes: u32,
}

#[derive(Clone, Copy)]
enum Operation {
    First,
    Next,
}

impl HardlinkLookup {
    pub(super) fn paths(&self, path: &str, volume: &str) -> Result<Vec<String>, ScanError> {
        let path = terminated(path)?;
        let mut name = vec![0_u16; MAX_PATH_UNITS];
        let mut count = name.len() as u32;
        // SAFETY: all buffers are bounded and live; this enumerates names only, never file data.
        let raw = unsafe { FindFirstFileNameW(path.as_ptr(), 0, &mut count, name.as_mut_ptr()) };
        if raw == INVALID_HANDLE_VALUE {
            return Err(self.failure(Operation::First, io::Error::last_os_error()));
        }
        let _search = LinkSearch(raw);
        let mut paths = BTreeSet::new();
        loop {
            let length = name
                .iter()
                .position(|unit| *unit == 0)
                .ok_or_else(invalid)?;
            let relative = String::from_utf16(&name[..length]).map_err(|_| invalid())?;
            if !relative.starts_with('\\') || paths.len() >= MAX_LINKS {
                return Err(invalid());
            }
            paths.insert(format!("{}{relative}", volume.trim_end_matches('\\')));
            name.fill(0);
            count = name.len() as u32;
            // SAFETY: the search owner and output buffer remain live at their declared capacities.
            if unsafe { FindNextFileNameW(raw, &mut count, name.as_mut_ptr()) } == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) {
                    break;
                }
                return Err(self.failure(Operation::Next, error));
            }
        }
        Ok(paths.into_iter().collect())
    }

    fn failure(&self, operation: Operation, error: io::Error) -> ScanError {
        let (api, ordinary_code) = match operation {
            Operation::First => ("FindFirstFileNameW", "identity_journal_hardlinks"),
            Operation::Next => ("FindNextFileNameW", "identity_journal_hardlink_next"),
        };
        let code = if error.raw_os_error() == Some(ERROR_NOT_SUPPORTED as i32) {
            "identity_journal_hardlinks_not_supported"
        } else {
            ordinary_code
        };
        let mut failure = metadata_error(code, error);
        // Persist enough identity evidence to locate a later failure without recording paths.
        failure.message = format!(
            "{}; operation={api}; identity={}; links={}; attributes={:#010x}",
            failure.message, self.identity.value, self.link_count, self.attributes,
        );
        failure
    }
}

struct LinkSearch(HANDLE);
impl Drop for LinkSearch {
    fn drop(&mut self) {
        // SAFETY: this owner retains the successful FindFirstFileNameW search handle.
        let _ = unsafe { FindClose(self.0) };
    }
}

#[cfg(test)]
mod tests;
