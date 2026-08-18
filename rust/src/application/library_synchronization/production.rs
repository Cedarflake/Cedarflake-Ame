#[cfg(windows)]
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use crate::adapters::{
    SqliteCatalog, inspect_root_availability, production_library_change_source_factory,
};
use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

#[cfg(windows)]
use super::LibrarySynchronizationRuntime;
#[cfg(windows)]
use crate::application::storage_paths;

#[cfg(windows)]
static SYNCHRONIZATION_RUNTIME: OnceLock<Mutex<Option<LibrarySynchronizationRuntime>>> =
    OnceLock::new();

pub(crate) fn start_production_library_synchronization()
-> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        let mut runtime_state = lock_runtime()?;
        let runtime = runtime_state.get_or_insert_with(|| {
            LibrarySynchronizationRuntime::new_erased(production_library_change_source_factory())
        });
        poll_runtime(runtime)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn poll_production_library_synchronization()
-> Result<LibrarySynchronizationSnapshot, ScanError> {
    #[cfg(windows)]
    {
        let mut runtime_state = lock_runtime()?;
        let runtime = runtime_state.as_mut().ok_or_else(|| {
            ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            )
        })?;
        poll_runtime(runtime)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_platform())
    }
}

pub(crate) fn stop_production_library_synchronization() -> Result<(), ScanError> {
    #[cfg(windows)]
    {
        let runtime_state = {
            let mut guarded = lock_runtime()?;
            guarded.take()
        };
        if let Some(mut runtime) = runtime_state {
            runtime.stop()?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(windows)]
fn poll_runtime(
    runtime: &mut LibrarySynchronizationRuntime,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    runtime.poll(&mut catalog, now_unix_ms()?, |root_path| {
        inspect_root_availability(root_path).availability
    })
}

#[cfg(windows)]
fn lock_runtime() -> Result<MutexGuard<'static, Option<LibrarySynchronizationRuntime>>, ScanError> {
    SYNCHRONIZATION_RUNTIME
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| {
            ScanError::new(
                "library_synchronization_state_unavailable",
                "The library synchronization runtime state is unavailable",
            )
        })
}

fn now_unix_ms() -> Result<i64, ScanError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is earlier than the Unix epoch",
        )
    })?;
    i64::try_from(elapsed.as_millis()).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is outside the supported range",
        )
    })
}

#[cfg(not(windows))]
fn unsupported_platform() -> ScanError {
    ScanError::new(
        "library_synchronization_unsupported",
        "Continuous library synchronization is currently supported only on Windows",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_is_representable() {
        assert!(now_unix_ms().expect("current time") > 0);
    }
}
