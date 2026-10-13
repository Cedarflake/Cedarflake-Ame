use windows_sys::Win32::Security::TokenImpersonation;

use super::*;

fn configured_identity() -> ProcessTokenIdentity {
    ProcessTokenIdentity {
        user_sid: LOCAL_SYSTEM_SID.to_owned(),
        token_type: TokenPrimary,
        is_elevated: true,
        is_restricted: true,
        restricted_sids: vec![SERVICE_SID.to_owned()],
    }
}

fn configured_privileges() -> Vec<String> {
    vec![
        VOLUME_PRIVILEGE.to_owned(),
        IMPERSONATION_PRIVILEGE.to_owned(),
        TRAVERSE_PRIVILEGE.to_owned(),
    ]
}

#[test]
fn service_start_before_restricted_sid_configuration_is_rejected() {
    let mut identity = configured_identity();
    identity.is_restricted = false;
    identity.restricted_sids.clear();
    assert!(validate_service_profile(&identity, &configured_privileges()).is_err());

    identity.is_restricted = true;
    identity
        .restricted_sids
        .push("S-1-5-80-1-2-3-4-5".to_owned());
    assert!(validate_service_profile(&identity, &configured_privileges()).is_err());
}

#[test]
fn service_start_before_privilege_configuration_is_rejected() {
    for extra in [
        "SeDebugPrivilege",
        "SeBackupPrivilege",
        "SeAssignPrimaryTokenPrivilege",
    ] {
        let mut privileges = configured_privileges();
        privileges.push(extra.to_owned());
        assert!(validate_service_profile(&configured_identity(), &privileges).is_err());
    }
    assert!(validate_service_profile(&configured_identity(), &[]).is_err());
    assert!(
        validate_service_profile(&configured_identity(), &[TRAVERSE_PRIVILEGE.to_owned()]).is_err()
    );
}

#[test]
fn configured_service_accepts_scm_retained_traverse_privilege_only_once() {
    assert!(validate_service_profile(&configured_identity(), &configured_privileges()).is_ok());
    assert!(
        validate_service_profile(
            &configured_identity(),
            &[
                VOLUME_PRIVILEGE.to_owned(),
                IMPERSONATION_PRIVILEGE.to_owned()
            ],
        )
        .is_ok()
    );
    for privilege in configured_privileges() {
        let mut duplicate = configured_privileges();
        duplicate.push(privilege);
        assert!(validate_service_profile(&configured_identity(), &duplicate).is_err());
    }
}

#[test]
fn service_requires_both_journal_access_and_caller_impersonation() {
    for missing in [VOLUME_PRIVILEGE, IMPERSONATION_PRIVILEGE] {
        let privileges = configured_privileges()
            .into_iter()
            .filter(|privilege| privilege != missing)
            .collect::<Vec<_>>();
        assert!(validate_service_profile(&configured_identity(), &privileges).is_err());
    }
}

#[test]
fn elevated_administrator_and_impersonation_tokens_do_not_establish_service_identity() {
    let mut administrator = configured_identity();
    administrator.user_sid = "S-1-5-21-1-2-3-1001".to_owned();
    assert!(validate_service_profile(&administrator, &configured_privileges()).is_err());

    let mut impersonation = configured_identity();
    impersonation.token_type = TokenImpersonation;
    assert!(validate_service_profile(&impersonation, &configured_privileges()).is_err());
}

#[test]
fn ordinary_process_cannot_start_the_production_service_host() {
    assert!(verify_running_service_profile().is_err());
}
