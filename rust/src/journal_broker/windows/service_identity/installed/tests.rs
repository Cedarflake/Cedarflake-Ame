use super::*;

fn valid_facts() -> ServerProcessFacts {
    ServerProcessFacts {
        scm_process_id: 42,
        pipe_process_id: 42,
        image_path: PathBuf::from(
            r"C:\Program Files\Cedarflake Ame\Journal Broker\cedarflake_ame_journal_broker.exe",
        ),
        expected_image_path: PathBuf::from(
            r"c:\PROGRAM FILES\Cedarflake Ame\Journal Broker\cedarflake_ame_journal_broker.exe",
        ),
        token_user_sid: LOCAL_SYSTEM_SID.to_owned(),
        token_type: TokenPrimary,
        is_elevated: true,
        is_restricted: true,
        restricted_sids: vec![SERVICE_SID.to_owned()],
        manifest: InstalledBrokerIdentity {
            protocol: PROTOCOL_VERSION,
            max_frame: MAX_FRAME_BYTES,
            binary_sha256: [5; 32],
        },
        actual_binary_sha256: [5; 32],
    }
}

#[test]
fn unsigned_broker_requires_protected_identity_and_os_process_facts() {
    assert!(valid_facts().validate().is_ok());
    type Mutation = Box<dyn Fn(&mut ServerProcessFacts)>;
    let mutations: Vec<Mutation> = vec![
        Box::new(|facts| facts.scm_process_id = 0),
        Box::new(|facts| facts.pipe_process_id += 1),
        Box::new(|facts| facts.image_path = PathBuf::from(r"C:\Temp\broker.exe")),
        Box::new(|facts| facts.token_user_sid = "S-1-5-21-1".to_owned()),
        Box::new(|facts| facts.token_type = windows_sys::Win32::Security::TokenImpersonation),
        Box::new(|facts| facts.is_elevated = false),
        Box::new(|facts| facts.is_restricted = false),
        Box::new(|facts| facts.restricted_sids.clear()),
        Box::new(|facts| facts.manifest.protocol += 1),
        Box::new(|facts| facts.manifest.max_frame += 1),
        Box::new(|facts| facts.actual_binary_sha256[0] ^= 1),
    ];
    for mutate in mutations {
        let mut facts = valid_facts();
        mutate(&mut facts);
        assert!(facts.validate().is_err());
    }
}

#[test]
fn manifest_version_is_exact_and_has_no_client_or_signature_requirement() {
    let digest = "A5".repeat(32);
    let text =
        format!("AMEJBID3|protocol={PROTOCOL_VERSION}|max_frame={MAX_FRAME_BYTES}|sha256={digest}");
    let parsed = InstalledBrokerIdentity::parse(&text).expect("unsigned service identity");
    assert_eq!(parsed.binary_sha256, [0xA5; 32]);
    for invalid in [
        text.replace("AMEJBID3", "AMEJBID2"),
        text.replace("|sha256=", "|extra=1|sha256="),
        text.replace("protocol=", "protocol=0"),
        text.replace("max_frame=", "max_frame=+"),
        text.replace(&digest, &digest[..62]),
        format!("{text}|client_path_utf16le=4300"),
        text.to_ascii_lowercase(),
    ] {
        assert!(InstalledBrokerIdentity::parse(&invalid).is_err());
    }
}
