use super::*;

fn lookup() -> HardlinkLookup {
    HardlinkLookup {
        identity: FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".to_owned(),
            value: "0000000000000007:00000000000000000000000000000003".to_owned(),
        },
        link_count: 2,
        attributes: 0x420,
    }
}

#[test]
fn unsupported_lookup_keeps_identity_and_the_exact_failing_operation() {
    for (operation, api) in [
        (Operation::First, "FindFirstFileNameW"),
        (Operation::Next, "FindNextFileNameW"),
    ] {
        let failure = lookup().failure(operation, io::Error::from_raw_os_error(50));
        assert_eq!(failure.code, "identity_journal_hardlinks_not_supported");
        assert!(
            failure
                .message
                .contains("Windows metadata operation failed: 50")
        );
        assert!(failure.message.contains(&format!("operation={api}")));
        assert!(
            failure
                .message
                .contains(&format!("identity={}", lookup().identity.value))
        );
        assert!(failure.message.contains("links=2; attributes=0x00000420"));
    }
}

#[test]
fn lookup_diagnostics_preserve_missing_and_inaccessible_evidence() {
    for operation in [Operation::First, Operation::Next] {
        for (error, code) in [
            (2, "identity_journal_identity_missing"),
            (3, "identity_journal_identity_missing"),
            (5, "identity_journal_identity_access_denied"),
            (32, "identity_journal_identity_access_denied"),
        ] {
            assert_eq!(
                lookup()
                    .failure(operation, io::Error::from_raw_os_error(error))
                    .code,
                code
            );
        }
    }
}
