use std::cell::Cell;

use super::*;

#[test]
fn malformed_pipe_names_fail_before_win32() {
    assert!(matches!(
        nul_terminated("bad\0name"),
        Err(WindowsTransportError::ProtocolMismatch)
    ));
}

#[test]
fn successful_connection_does_not_probe_or_restart_the_service() {
    let result = connect_with_handover(
        || Ok(42),
        |process| Ok(*process),
        |_| panic!("a successful connection cannot request a handover"),
    );
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn live_server_authentication_failure_is_not_retried() {
    let starts = Cell::new(0);
    let result = connect_with_handover(
        || {
            starts.set(starts.get() + 1);
            Ok(42)
        },
        |_| Err::<(), _>(WindowsTransportError::ServerIdentityRejected),
        |_| Ok(false),
    );
    assert!(matches!(
        result,
        Err(WindowsTransportError::ServerIdentityRejected)
    ));
    assert_eq!(starts.get(), 1);
}

#[test]
fn unverified_retirement_preserves_the_first_connection_failure() {
    let result = connect_with_handover(
        || Ok(42),
        |_| Err::<(), _>(WindowsTransportError::ServerIdentityRejected),
        |_| Err(WindowsTransportError::TimedOut),
    );
    assert!(matches!(
        result,
        Err(WindowsTransportError::ServerIdentityRejected)
    ));
}

#[test]
fn verified_service_handover_repeats_full_connection_once() {
    let starts = Cell::new(41);
    let connections = Cell::new(0);
    let result = connect_with_handover(
        || {
            starts.set(starts.get() + 1);
            Ok(starts.get())
        },
        |process| {
            connections.set(connections.get() + 1);
            if *process == 42 {
                Err(WindowsTransportError::Closed)
            } else {
                Ok(*process)
            }
        },
        |_| Ok(true),
    );
    assert_eq!(result.unwrap(), 43);
    assert_eq!(connections.get(), 2);
}

#[test]
fn failed_replacement_has_no_third_connection_attempt() {
    let starts = Cell::new(0);
    let connections = Cell::new(0);
    let result = connect_with_handover(
        || {
            starts.set(starts.get() + 1);
            Ok(starts.get())
        },
        |_| {
            connections.set(connections.get() + 1);
            Err::<(), _>(WindowsTransportError::ServerIdentityRejected)
        },
        |_| Ok(true),
    );
    assert!(matches!(
        result,
        Err(WindowsTransportError::ServerIdentityRejected)
    ));
    assert_eq!(starts.get(), 2);
    assert_eq!(connections.get(), 2);
}
