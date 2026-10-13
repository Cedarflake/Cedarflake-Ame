use super::super::{file_identity, open_directory_handle};
use super::*;

mod child_process;

use child_process::OwnedCallerChild;

fn token_evidence(primary: &PrimaryTokenEvidence) -> TokenEvidence {
    TokenEvidence {
        user_sid: primary.user_sid.clone(),
        session_id: primary.session_id,
        authentication_id: primary.authentication_id,
        modified_id: LUID {
            LowPart: 1,
            HighPart: 0,
        },
        token_type: windows_sys::Win32::Security::TokenImpersonation,
        impersonation_level: windows_sys::Win32::Security::SecurityImpersonation,
        is_elevated: primary.is_elevated,
    }
}

#[test]
fn primary_process_and_impersonated_caller_must_agree() {
    let caller =
        PipeCallerProcess::open(std::process::id()).expect("current unsigned test process");
    let primary = caller.primary();
    assert!(primary.matches_impersonation(&token_evidence(primary)));
    type Mutation = Box<dyn Fn(&mut TokenEvidence)>;
    let mutations: Vec<Mutation> = vec![
        Box::new(|facts| facts.user_sid.push(1)),
        Box::new(|facts| facts.session_id += 1),
        Box::new(|facts| facts.authentication_id.LowPart ^= 1),
        Box::new(|facts| facts.authentication_id.HighPart ^= 1),
        Box::new(|facts| facts.is_elevated = !facts.is_elevated),
    ];
    for mutate in mutations {
        let mut evidence = token_evidence(primary);
        mutate(&mut evidence);
        assert!(!primary.matches_impersonation(&evidence));
    }
}

#[test]
fn process_identity_pins_lifetime_and_duplicates_only_the_held_callers_handle() {
    let caller = PipeCallerProcess::open(std::process::id()).expect("current process");
    assert_ne!(caller.creation_time, 0);
    assert_ne!(caller.opaque_identity(), [0; 32]);
    assert_ne!(
        caller.opaque_identity(),
        process_identity(
            caller.process_id,
            caller.creation_time + 1,
            caller.primary()
        )
    );
    assert_ne!(
        caller.opaque_identity(),
        process_identity(
            caller.process_id + 1,
            caller.creation_time,
            caller.primary()
        )
    );
    assert!(PipeCallerProcess::open(0).is_err());
    assert!(caller.duplicate_root_handle(0).is_err());
    assert!(caller.duplicate_root_handle(u64::MAX).is_err());

    let directory = tempfile::tempdir().expect("generated directory");
    let path = directory
        .path()
        .to_str()
        .unwrap()
        .encode_utf16()
        .collect::<Vec<_>>();
    let original = open_directory_handle(&path).expect("directory handle");
    let duplicate = caller
        .duplicate_root_handle(original.raw() as usize as u64)
        .expect("caller-owned directory duplicate");
    let identity = file_identity(original.raw()).expect("original identity");
    drop(original);
    assert_eq!(
        file_identity(duplicate.raw()).expect("retained duplicate identity"),
        identity
    );
}

#[test]
fn held_caller_rejects_operations_after_the_process_exits() {
    if child_process::run_child_if_requested() {
        return;
    }
    let mut child = OwnedCallerChild::spawn();
    let caller = PipeCallerProcess::open(child.process_id()).expect("owned child process");
    let source_handle = child.wait_for_root_handle();
    caller.ensure_alive().expect("child remains alive");
    let duplicate = caller
        .duplicate_root_handle(source_handle)
        .expect("live child's root handle");
    let identity = file_identity(duplicate.raw()).expect("duplicated root identity");

    child.request_exit_and_wait();
    assert!(matches!(
        caller.ensure_alive(),
        Err(WindowsSecurityError::CallerRejected)
    ));
    assert!(matches!(
        caller.duplicate_root_handle(source_handle),
        Err(WindowsSecurityError::CallerRejected)
    ));
    assert_eq!(
        file_identity(duplicate.raw()).expect("root duplicate survives caller exit"),
        identity
    );
}
