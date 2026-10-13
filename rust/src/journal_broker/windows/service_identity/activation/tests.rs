use windows_sys::Win32::System::Services::SERVICE_PAUSED;

use super::*;

fn status(state: u32, process_id: u32) -> SERVICE_STATUS_PROCESS {
    SERVICE_STATUS_PROCESS {
        dwCurrentState: state,
        dwProcessId: process_id,
        ..Default::default()
    }
}

#[test]
fn demand_start_waits_for_idle_stop_before_starting_a_new_process() {
    let mut activation = Activation::default();
    assert_eq!(
        activation
            .observe(&status(SERVICE_STOP_PENDING, 42))
            .unwrap(),
        NextStep::Wait
    );
    assert_eq!(
        activation.observe(&status(SERVICE_STOPPED, 0)).unwrap(),
        NextStep::Start
    );
    activation.started = true;
    assert_eq!(
        activation
            .observe(&status(SERVICE_START_PENDING, 43))
            .unwrap(),
        NextStep::Wait
    );
    assert_eq!(
        activation.observe(&status(SERVICE_RUNNING, 43)).unwrap(),
        NextStep::Ready(43)
    );
}

#[test]
fn another_clients_start_is_observed_without_an_extra_start_request() {
    let mut activation = Activation::default();
    assert_eq!(
        activation
            .observe(&status(SERVICE_START_PENDING, 42))
            .unwrap(),
        NextStep::Wait
    );
    assert_eq!(
        activation.observe(&status(SERVICE_RUNNING, 42)).unwrap(),
        NextStep::Ready(42)
    );
}

#[test]
fn failed_service_start_cannot_be_retried_into_apparent_success() {
    let mut activation = Activation { started: true };
    let mut stopped = status(SERVICE_STOPPED, 0);
    stopped.dwWin32ExitCode = 1066;
    assert!(
        matches!(activation.observe(&stopped), Err(WindowsTransportError::Io(error)) if error.raw_os_error() == Some(1066))
    );
    assert!(activation.observe(&status(SERVICE_STOPPED, 0)).is_err());
    assert!(activation.observe(&status(SERVICE_RUNNING, 0)).is_err());
    assert!(activation.observe(&status(SERVICE_PAUSED, 42)).is_err());
}

#[test]
fn another_clients_failed_start_is_not_restarted() {
    let mut activation = Activation::default();
    assert_eq!(
        activation
            .observe(&status(SERVICE_START_PENDING, 42))
            .unwrap(),
        NextStep::Wait
    );
    let mut stopped = status(SERVICE_STOPPED, 0);
    stopped.dwWin32ExitCode = 1066;
    assert!(
        matches!(activation.observe(&stopped), Err(WindowsTransportError::Io(error)) if error.raw_os_error() == Some(1066))
    );
}

#[test]
fn only_a_verified_process_transition_admits_connection_handover() {
    assert!(!process_has_retired(&status(SERVICE_RUNNING, 42), 42));
    assert!(!process_has_retired(&status(SERVICE_RUNNING, 0), 42));
    assert!(!process_has_retired(&status(SERVICE_PAUSED, 42), 42));
    assert!(process_has_retired(&status(SERVICE_STOP_PENDING, 42), 42));
    assert!(process_has_retired(&status(SERVICE_STOPPED, 0), 42));
    assert!(process_has_retired(&status(SERVICE_START_PENDING, 43), 42));
    assert!(process_has_retired(&status(SERVICE_RUNNING, 43), 42));
}

#[test]
fn retirement_observation_allows_scm_to_catch_up_with_pipe_shutdown() {
    let mut observations = 0;
    let retired = observe_retirement(
        42,
        || {
            observations += 1;
            if observations == 1 {
                Ok(status(SERVICE_RUNNING, 42))
            } else {
                Ok(status(SERVICE_STOP_PENDING, 42))
            }
        },
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert!(retired);
    assert_eq!(observations, 2);
}

#[test]
fn live_process_failure_never_creates_retirement_evidence() {
    let mut observations = 0;
    let retired = observe_retirement(
        42,
        || {
            observations += 1;
            Ok(status(SERVICE_RUNNING, 42))
        },
        Instant::now(),
    )
    .unwrap();
    assert!(!retired);
    assert_eq!(observations, 1);
}
