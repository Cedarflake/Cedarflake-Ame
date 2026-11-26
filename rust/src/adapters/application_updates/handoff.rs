use std::fs::{self, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::package::{RetainedPackage, cache_directory, reject_reparse, storage};
use crate::domain::application_update::{ApplicationUpdateError, ApplicationUpdateFailureCode};

mod process;
use process::{Activation, HandoffProcess, Purpose};

const HELPER: &str = "cedarflake_ame_update.exe";

pub(super) fn is_installed(application: &Path) -> Result<bool, ApplicationUpdateError> {
    let helper = application.with_file_name(HELPER);
    if !helper.is_file() {
        return Ok(false);
    }
    reject_reparse(&helper)?;
    let mut child = hidden_command(&helper)
        .arg("--probe")
        .arg(application)
        .spawn()
        .map_err(storage)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let exit = match child.try_wait() {
            Ok(exit) => exit,
            Err(error) => {
                retire(&mut child);
                return Err(storage(error));
            }
        };
        if let Some(exit) = exit {
            return match exit.code() {
                Some(0) => Ok(true),
                Some(1) => Ok(false),
                _ => Err(installation(
                    "Installed-product identity could not be verified",
                )),
            };
        }
        if Instant::now() >= deadline {
            retire(&mut child);
            return Err(installation("Installed-product inspection timed out"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn prepare(
    application: &Path,
    package: &RetainedPackage,
    version: &str,
    portable: bool,
) -> Result<PreparedHandoff, ApplicationUpdateError> {
    let cache = cache_directory()?;
    reject_reparse(&cache)?;
    fs::create_dir_all(&cache).map_err(storage)?;
    cleanup_retired_handoffs(&cache);
    let directory = tempfile::Builder::new()
        .prefix("handoff-")
        .tempdir_in(&cache)
        .map_err(storage)?;
    let helper = directory.path().join(HELPER);
    let shipped_helper = application.with_file_name(HELPER);
    reject_reparse(&shipped_helper)?;
    fs::copy(shipped_helper, &helper).map_err(storage)?;
    let ready = directory.path().join("ready");
    let result = cache.join("result.json");
    let engine = if portable {
        Some(super::portable_engine::stage(
            package,
            version,
            directory.path(),
        )?)
    } else {
        None
    };
    let child = hidden_command(&helper)
        .arg(if portable { "--portable" } else { "--apply" })
        .arg(std::process::id().to_string())
        .arg(application)
        .arg(&package.path)
        .arg(&package.asset.sha256)
        .arg(package.asset.size.to_string())
        .arg(&ready)
        .arg(result)
        .arg(version)
        .arg(engine.as_ref().map_or("", |engine| engine.sha256.as_str()))
        .arg(
            engine
                .as_ref()
                .map_or(0, |engine| engine.length)
                .to_string(),
        )
        .spawn()
        .map_err(storage)?;
    let mut process = HandoffProcess::new(child, directory);
    let readiness = process.acknowledge(Purpose::Update);
    drop(engine);
    readiness?;
    Ok(PreparedHandoff {
        process: Mutex::new(process),
        application: application.to_owned(),
        version: version.to_owned(),
    })
}

pub(super) struct PreparedHandoff {
    process: Mutex<HandoffProcess>,
    application: PathBuf,
    version: String,
}

impl PreparedHandoff {
    pub fn commit(&self) -> Result<(), ApplicationUpdateError> {
        let mut process = self.process.lock().map_err(storage)?;
        match process.activate()? {
            Activation::Committed => Ok(()),
            Activation::RecoveryArmed => Err(installation(
                "The previous application will reopen because update activation failed",
            )),
            Activation::RecoveryRequired => {
                let recovery = prepare_recovery(
                    &process.directory().join(HELPER),
                    &self.application,
                    &self.version,
                )?;
                process.replace_with_recovery(recovery);
                Err(installation(
                    "The update handoff exited; a replacement helper will reopen the previous application",
                ))
            }
        }
    }
}

fn prepare_recovery(
    helper_source: &Path,
    application: &Path,
    version: &str,
) -> Result<HandoffProcess, ApplicationUpdateError> {
    let cache = helper_source
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| installation("Recovery cache is unavailable"))?;
    reject_reparse(helper_source)?;
    let directory = tempfile::Builder::new()
        .prefix("handoff-")
        .tempdir_in(cache)
        .map_err(storage)?;
    let helper = directory.path().join(HELPER);
    fs::copy(helper_source, &helper).map_err(storage)?;
    let child = hidden_command(&helper)
        .arg("--recover")
        .arg(std::process::id().to_string())
        .arg(application)
        .arg(directory.path().join("ready"))
        .arg(cache.join("result.json"))
        .arg(version)
        .spawn()
        .map_err(|error| {
            installation(&format!(
                "Cannot start previous-application recovery: {error}"
            ))
        })?;
    let mut recovery = HandoffProcess::new(child, directory);
    recovery.acknowledge(Purpose::Recovery)?;
    Ok(recovery)
}

fn cleanup_retired_handoffs(cache: &Path) {
    let Ok(entries) = fs::read_dir(cache) else {
        return;
    };
    for entry in entries.take(64).flatten() {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let old = metadata
            .modified()
            .ok()
            .and_then(|time| time.elapsed().ok())
            .is_some_and(|age| age >= Duration::from_secs(24 * 60 * 60));
        if !entry.file_name().to_string_lossy().starts_with("handoff-")
            || !metadata.is_dir()
            || metadata.file_attributes() & 0x400 != 0
            || !old
        {
            continue;
        }
        if reject_reparse(&path.join("lease")).is_err() {
            continue;
        }
        let Ok(lease) = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(path.join("lease"))
        else {
            continue;
        };
        let Ok(children) = fs::read_dir(&path) else {
            continue;
        };
        let children: Vec<_> = children.take(8).collect();
        let allowed = [
            HELPER,
            "lease",
            "ready",
            "commit",
            "ready.tmp",
            "result.tmp",
            "Update.exe",
        ];
        if children.len() > allowed.len()
            || children.iter().any(|child| match child {
                Ok(child) => {
                    !allowed.iter().any(|name| child.file_name() == *name)
                        || fs::symlink_metadata(child.path()).map_or(true, |metadata| {
                            !metadata.is_file() || metadata.file_attributes() & 0x400 != 0
                        })
                }
                Err(_) => true,
            })
        {
            continue;
        }
        // An active helper retains the lease; never recursively adopt unknown cache contents.
        for child in children.into_iter().flatten() {
            if child.file_name() != "lease" {
                let _ = fs::remove_file(child.path());
            }
        }
        drop(lease);
        let _ = fs::remove_file(path.join("lease"));
        let _ = fs::remove_dir(path);
    }
}

fn hidden_command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn retire(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn installation(detail: &str) -> ApplicationUpdateError {
    ApplicationUpdateError::new(ApplicationUpdateFailureCode::Installation, detail)
}
