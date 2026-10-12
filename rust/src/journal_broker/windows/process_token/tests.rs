use super::*;

#[test]
fn privilege_parser_retains_disabled_privileges_and_rejects_truncated_entries() {
    let offset = offset_of!(TOKEN_PRIVILEGES, Privileges);
    let mut bytes = vec![0_u8; offset + size_of::<LUID_AND_ATTRIBUTES>()];
    bytes[..4].copy_from_slice(&1_u32.to_ne_bytes());
    let entry = LUID_AND_ATTRIBUTES {
        Luid: LUID {
            LowPart: 17,
            HighPart: 0,
        },
        Attributes: 0,
    };
    // SAFETY: the fixture reserves one complete entry and uses an unaligned write.
    unsafe {
        bytes
            .as_mut_ptr()
            .add(offset)
            .cast::<LUID_AND_ATTRIBUTES>()
            .write_unaligned(entry)
    };
    let parsed = token_privilege_ids(&bytes).expect("complete privilege");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].LowPart, 17);
    assert!(token_privilege_ids(&bytes[..bytes.len() - 1]).is_err());
    bytes[..4].copy_from_slice(&u32::MAX.to_ne_bytes());
    assert!(token_privilege_ids(&bytes).is_err());
}

#[test]
fn restricted_sid_parser_rejects_pointers_outside_its_query_buffer() {
    let offset = offset_of!(TOKEN_GROUPS, Groups);
    let mut bytes =
        vec![0_u8; offset + size_of::<windows_sys::Win32::Security::SID_AND_ATTRIBUTES>()];
    bytes[..4].copy_from_slice(&1_u32.to_ne_bytes());
    let entry = windows_sys::Win32::Security::SID_AND_ATTRIBUTES {
        Sid: null_mut(),
        Attributes: 0,
    };
    // SAFETY: the fixture reserves one complete group entry; its null SID must be rejected.
    unsafe {
        bytes
            .as_mut_ptr()
            .add(offset)
            .cast::<windows_sys::Win32::Security::SID_AND_ATTRIBUTES>()
            .write_unaligned(entry)
    };
    assert!(token_group_sids_from_bytes(&bytes).is_err());
    assert!(token_group_sids_from_bytes(&bytes[..bytes.len() - 1]).is_err());
}

#[test]
fn current_process_token_can_be_read_without_mutation_or_elevation() {
    // SAFETY: this borrowed pseudo-handle refers to the running test process.
    let process = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
    let token = ProcessToken::open(process).expect("query own token");
    let identity = token.identity().expect("own token identity");
    assert!(identity.user_sid.starts_with("S-1-"));
    assert_eq!(
        identity.token_type,
        windows_sys::Win32::Security::TokenPrimary
    );
    assert!(!token.privilege_names().expect("own privileges").is_empty());
}

fn write_token_user_header(bytes: &mut [u8], sid_offset: usize) {
    let sid = unsafe { bytes.as_mut_ptr().add(sid_offset) }.cast();
    let header = TOKEN_USER {
        User: windows_sys::Win32::Security::SID_AND_ATTRIBUTES {
            Sid: sid,
            Attributes: 0,
        },
    };
    // SAFETY: tests reserve a complete TOKEN_USER header and deliberately exercise unaligned
    // storage; write_unaligned initializes it without forming an unaligned reference.
    unsafe {
        bytes
            .as_mut_ptr()
            .cast::<TOKEN_USER>()
            .write_unaligned(header)
    };
}

fn local_system_sid_bytes() -> [u8; 12] {
    [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0]
}

#[test]
fn token_user_sid_parser_accepts_unaligned_complete_buffer() {
    let sid_offset = size_of::<TOKEN_USER>();
    let sid = local_system_sid_bytes();
    let mut allocation = vec![0_u8; 1 + sid_offset + sid.len()];
    let bytes = &mut allocation[1..];
    assert_ne!(
        bytes.as_ptr() as usize % std::mem::align_of::<TOKEN_USER>(),
        0
    );
    bytes[sid_offset..].copy_from_slice(&sid);
    write_token_user_header(bytes, sid_offset);

    assert_eq!(token_user_sid_slice(bytes).expect("valid SID"), sid);
}

#[test]
fn token_user_sid_parser_rejects_out_of_buffer_and_malformed_sid() {
    let sid_offset = size_of::<TOKEN_USER>();
    let sid = local_system_sid_bytes();
    let mut complete = vec![0_u8; sid_offset + sid.len()];
    complete[sid_offset..].copy_from_slice(&sid);
    write_token_user_header(&mut complete, sid_offset);
    assert!(token_user_sid_slice(&complete[..complete.len() - 1]).is_err());

    let mut outside = vec![0_u8; sid_offset + sid.len()];
    let outside_offset = outside.len();
    write_token_user_header(&mut outside, outside_offset);
    assert!(token_user_sid_slice(&outside).is_err());

    let mut short_header = vec![0_u8; sid_offset + 7];
    write_token_user_header(&mut short_header, sid_offset);
    assert!(token_user_sid_slice(&short_header).is_err());

    let mut malformed = vec![0_u8; sid_offset + sid.len()];
    malformed[sid_offset..].copy_from_slice(&sid);
    malformed[sid_offset] = 0;
    write_token_user_header(&mut malformed, sid_offset);
    assert!(token_user_sid_slice(&malformed).is_err());
}
