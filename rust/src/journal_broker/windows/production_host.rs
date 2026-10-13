mod lifetime;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows_sys::Win32::System::Pipes::DisconnectNamedPipe;

use lifetime::ServiceLifetime;

use super::connection::{ConnectionIdentitySource, ProcessConnectionIdentitySource};
use super::host::{
    HostError, MAX_HOST_READ_WORKERS, ReadWorkerBudget, connect_bounded, serve_named_pipe,
};
use super::identity_query_access::prepare_current_service;
use super::pipe_listener::{MAX_PRODUCTION_CONNECTIONS, create_pipe, production_pipe_sddl};
use super::transport::{OwnedHandle, PRODUCTION_PIPE_NAME};
use crate::journal_broker::service::WindowsRootRegistrationBudget;

const SERVICE_IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const SERVICE_POLL_INTERVAL: Duration = Duration::from_millis(25);
const LISTENER_POLL_TIMEOUT: Duration = Duration::from_millis(250);

pub(super) fn run_with_ready(
    stop_requested: &'static AtomicBool,
    ready: SyncSender<()>,
) -> Result<(), ()> {
    prepare_current_service().map_err(|_| ())?;
    let first = create_listener(true).map_err(|_| ())?;
    let second = create_listener(false).map_err(|_| ())?;
    let identities = Arc::new(ProcessConnectionIdentitySource::new());
    let root_budget = WindowsRootRegistrationBudget::production();
    let worker_budget = ReadWorkerBudget::new(MAX_HOST_READ_WORKERS);
    let lifetime = ServiceLifetime::new(
        Instant::now(),
        SERVICE_IDLE_TIMEOUT,
        MAX_PRODUCTION_CONNECTIONS,
    );
    let mut listeners = Vec::with_capacity(MAX_PRODUCTION_CONNECTIONS as usize);
    for initial in [first, second] {
        let identities = Arc::clone(&identities);
        let root_budget = root_budget.clone();
        let worker_budget = worker_budget.clone();
        let lifetime = lifetime.clone();
        listeners.push(std::thread::spawn(move || {
            run_listener(
                initial,
                stop_requested,
                identities,
                root_budget,
                worker_budget,
                lifetime,
            )
        }));
    }
    let mut failed = ready.send(()).is_err();
    if failed {
        stop_requested.store(true, Ordering::Release);
    }
    while !stop_requested.load(Ordering::Acquire) {
        if listeners.iter().any(JoinHandle::is_finished) {
            failed = true;
            stop_requested.store(true, Ordering::Release);
            break;
        }
        match lifetime.retire_if_idle(Instant::now()) {
            Ok(false) => {}
            Ok(true) => stop_requested.store(true, Ordering::Release),
            Err(_) => {
                failed = true;
                stop_requested.store(true, Ordering::Release);
            }
        }
        std::thread::sleep(SERVICE_POLL_INTERVAL);
    }
    for listener in listeners {
        match listener.join() {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => failed = true,
        }
    }
    if failed { Err(()) } else { Ok(()) }
}

fn create_listener(first: bool) -> Result<OwnedHandle, HostError> {
    create_pipe(
        PRODUCTION_PIPE_NAME,
        &production_pipe_sddl(),
        false,
        first,
        MAX_PRODUCTION_CONNECTIONS,
    )
}

fn run_listener(
    initial: OwnedHandle,
    stop_requested: &'static AtomicBool,
    identities: Arc<ProcessConnectionIdentitySource>,
    root_budget: WindowsRootRegistrationBudget,
    worker_budget: ReadWorkerBudget,
    lifetime: ServiceLifetime,
) -> Result<(), HostError> {
    let mut pipe = Some(initial);
    while !stop_requested.load(Ordering::Acquire) {
        let current = match pipe.take() {
            Some(initial) => initial,
            None => create_listener(false)?,
        };
        match connect_bounded(&current, Instant::now() + LISTENER_POLL_TIMEOUT) {
            Ok(()) => {
                if stop_requested.load(Ordering::Acquire) {
                    break;
                }
                let Some(connection) = lifetime.admit().map_err(|_| HostError::DispatchFailed)?
                else {
                    break;
                };
                let identity = identities.next_identity()?;
                let _ = serve_named_pipe(
                    &current,
                    identity,
                    Some(stop_requested),
                    root_budget.clone(),
                    worker_budget.clone(),
                );
                // SAFETY: serving has retired its I/O and drained all request workers. Keep the
                // admission lease until the connected pipe is also detached.
                let _ = unsafe { DisconnectNamedPipe(current.raw()) };
                drop(connection);
            }
            Err(HostError::TimedOut) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
