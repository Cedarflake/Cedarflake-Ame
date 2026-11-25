use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use super::{installation, reject_reparse, retire, storage};
use crate::domain::application_update::ApplicationUpdateError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Authority {
    Preparing,
    Prepared,
    Committed,
    Recovery,
    Retired,
}

pub(super) enum Purpose {
    Update,
    Recovery,
}

pub(super) enum Activation {
    Committed,
    RecoveryRequired,
    RecoveryArmed,
}

pub(super) struct HandoffProcess {
    child: Child,
    directory: Option<tempfile::TempDir>,
    authority: Authority,
}

impl HandoffProcess {
    pub fn new(child: Child, directory: tempfile::TempDir) -> Self {
        Self {
            child,
            directory: Some(directory),
            authority: Authority::Preparing,
        }
    }

    pub fn directory(&self) -> &Path {
        self.directory
            .as_ref()
            .expect("Owned handoff directory")
            .path()
    }

    pub fn acknowledge(&mut self, purpose: Purpose) -> Result<(), ApplicationUpdateError> {
        let ready = self.directory().join("ready");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if self.child.try_wait().map_err(storage)?.is_some() {
                return Err(installation(
                    "The update handoff exited before acknowledging readiness",
                ));
            }
            if ready.is_file() {
                reject_reparse(&ready)?;
                let mut bytes = Vec::with_capacity(6);
                OpenOptions::new()
                    .read(true)
                    .share_mode(1)
                    .open(&ready)
                    .map_err(storage)?
                    .take(6)
                    .read_to_end(&mut bytes)
                    .map_err(storage)?;
                if bytes != b"ready" {
                    return Err(installation(
                        "The update handoff acknowledgement is invalid",
                    ));
                }
                self.authority = match purpose {
                    Purpose::Update => Authority::Prepared,
                    Purpose::Recovery => Authority::Recovery,
                };
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(installation("The update handoff timed out"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn activate(&mut self) -> Result<Activation, ApplicationUpdateError> {
        if matches!(self.authority, Authority::Preparing | Authority::Retired) {
            return Err(installation(
                "The update handoff has no exit recovery authority",
            ));
        }
        if self.child.try_wait().map_err(storage)?.is_some() {
            return Ok(Activation::RecoveryRequired);
        }
        match self.authority {
            Authority::Committed => return Ok(Activation::Committed),
            Authority::Recovery => return Ok(Activation::RecoveryArmed),
            _ => {}
        }
        reject_reparse(self.directory())?;
        let commit = self.directory().join("commit");
        let mut temporary = tempfile::NamedTempFile::new_in(self.directory()).map_err(storage)?;
        temporary.write_all(b"apply").map_err(storage)?;
        temporary.as_file().sync_all().map_err(storage)?;
        temporary.persist_noclobber(&commit).map_err(storage)?;
        if self.child.try_wait().map_err(storage)?.is_some() {
            // A recovery-only replacement ignores this marker even if its removal fails.
            let _ = fs::remove_file(commit);
            return Ok(Activation::RecoveryRequired);
        }
        self.authority = Authority::Committed;
        Ok(Activation::Committed)
    }

    pub fn replace_with_recovery(&mut self, replacement: Self) {
        self.authority = Authority::Retired;
        *self = replacement;
    }
}

impl Drop for HandoffProcess {
    fn drop(&mut self) {
        match self.authority {
            Authority::Preparing | Authority::Retired => retire(&mut self.child),
            Authority::Prepared | Authority::Committed | Authority::Recovery => {
                // Readiness transfers fallback ownership to the helper, independently of apply.
                if let Some(directory) = self.directory.take() {
                    let _ = directory.keep();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
