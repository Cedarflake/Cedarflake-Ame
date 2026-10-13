use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    ACE_HEADER, AccessCheck, CreateRestrictedToken, DISABLE_MAX_PRIVILEGE, DuplicateTokenEx,
    EqualSid, GENERIC_MAPPING, GetAce, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    InitializeSecurityDescriptor, PRIVILEGE_SET, SECURITY_DESCRIPTOR, SID_AND_ATTRIBUTES,
    SecurityImpersonation, SetSecurityDescriptorDacl, SetSecurityDescriptorGroup,
    SetSecurityDescriptorOwner, TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_GROUPS, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_ADJUST_SESSIONID, TOKEN_ALL_ACCESS, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
    TOKEN_IMPERSONATE, TokenImpersonation,
};
use windows_sys::Win32::Storage::FileSystem::WRITE_OWNER;
use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
use windows_sys::Win32::System::Threading::{
    PROCESS_ALL_ACCESS, PROCESS_CREATE_THREAD, PROCESS_DUP_HANDLE, PROCESS_QUERY_INFORMATION,
    PROCESS_TERMINATE, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

use super::*;

const SERVICE_DACL: &str = "O:SYG:SYD:P(A;;GA;;;SY)(A;;GA;;;BA)";

#[test]
fn ordinary_identity_queries_need_an_explicit_process_query_grant() {
    let original = descriptor(SERVICE_DACL);
    let original_dacl = descriptor_dacl(&original);
    let caller = restricted_interactive_caller();
    assert!(!allows(
        original_dacl,
        &caller,
        IdentityObject::Process,
        PROCESS_QUERY_LIMITED_INFORMATION
    ));

    let replacement = identity_query_acl(original_dacl, IdentityObject::Process).unwrap();
    assert!(allows(
        replacement.0.cast(),
        &caller,
        IdentityObject::Process,
        PROCESS_QUERY_LIMITED_INFORMATION
    ));
    for denied in [
        PROCESS_ALL_ACCESS,
        PROCESS_QUERY_INFORMATION,
        PROCESS_VM_READ,
        PROCESS_VM_WRITE,
        PROCESS_VM_OPERATION,
        PROCESS_DUP_HANDLE,
        PROCESS_TERMINATE,
        PROCESS_CREATE_THREAD,
        READ_CONTROL,
        WRITE_DAC,
        WRITE_OWNER,
    ] {
        assert!(
            !allows(
                replacement.0.cast(),
                &caller,
                IdentityObject::Process,
                denied
            ),
            "unexpected process access {denied:#x}"
        );
    }
    assert!(!allows(
        original_dacl,
        &caller,
        IdentityObject::Process,
        PROCESS_QUERY_LIMITED_INFORMATION
    ));
}

#[test]
fn identity_token_queries_never_grant_duplication_impersonation_or_mutation() {
    let original = descriptor(SERVICE_DACL);
    let original_dacl = descriptor_dacl(&original);
    let caller = restricted_interactive_caller();
    assert!(!allows(
        original_dacl,
        &caller,
        IdentityObject::Token,
        TOKEN_QUERY
    ));

    let replacement = identity_query_acl(original_dacl, IdentityObject::Token).unwrap();
    assert!(allows(
        replacement.0.cast(),
        &caller,
        IdentityObject::Token,
        TOKEN_QUERY
    ));
    for denied in [
        TOKEN_ALL_ACCESS,
        TOKEN_DUPLICATE,
        TOKEN_IMPERSONATE,
        TOKEN_ASSIGN_PRIMARY,
        TOKEN_ADJUST_DEFAULT,
        TOKEN_ADJUST_GROUPS,
        TOKEN_ADJUST_PRIVILEGES,
        TOKEN_ADJUST_SESSIONID,
        READ_CONTROL,
        WRITE_DAC,
        WRITE_OWNER,
    ] {
        assert!(
            !allows(replacement.0.cast(), &caller, IdentityObject::Token, denied),
            "unexpected token access {denied:#x}"
        );
    }
}

#[test]
fn identity_query_grants_preserve_existing_denials_and_administrator_entries() {
    let original = descriptor("O:SYG:SYD:P(D;;0x1000;;;AN)(A;;GA;;;SY)(A;;GA;;;BA)");
    let original_entries = ace_bytes(descriptor_dacl(&original));
    let replacement =
        identity_query_acl(descriptor_dacl(&original), IdentityObject::Process).unwrap();
    let replacement_entries = ace_bytes(replacement.0.cast());
    assert_eq!(replacement_entries.len(), original_entries.len() + 1);
    for entry in original_entries {
        assert!(replacement_entries.contains(&entry));
    }
}

#[test]
fn null_dacl_cannot_be_relabelled_as_a_constrained_query_grant() {
    assert!(matches!(
        identity_query_acl(null(), IdentityObject::Process),
        Err(WindowsTransportError::ServerIdentityRejected)
    ));
}

#[test]
fn native_token_query_grant_preserves_owner_and_existing_permissions() {
    let original = current_token();
    let mut duplicate = null_mut();
    // SAFETY: only this new token can be reconfigured. Neither the primary process token nor
    // any thread's effective token is replaced or impersonated by this fixture.
    assert_ne!(
        unsafe {
            DuplicateTokenEx(
                original.raw(),
                TOKEN_QUERY | READ_CONTROL | WRITE_DAC,
                null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut duplicate,
            )
        },
        0
    );
    let token = OwnedHandle::new(duplicate).unwrap();
    let before = kernel_descriptor(token.raw());
    let before_entries = ace_bytes(descriptor_dacl(&before));
    grant_identity_queries(token.raw(), IdentityObject::Token).unwrap();
    let after = kernel_descriptor(token.raw());
    let after_entries = ace_bytes(descriptor_dacl(&after));
    // SAFETY: both descriptor allocations retain the owner SIDs during this comparison.
    assert_ne!(
        unsafe { EqualSid(descriptor_owner(&before), descriptor_owner(&after)) },
        0
    );
    for entry in before_entries {
        assert!(after_entries.contains(&entry));
    }
    let caller = restricted_interactive_caller();
    assert!(allows(
        descriptor_dacl(&after),
        &caller,
        IdentityObject::Token,
        TOKEN_QUERY
    ));
    assert!(!allows(
        descriptor_dacl(&after),
        &caller,
        IdentityObject::Token,
        TOKEN_DUPLICATE
    ));
}

#[test]
fn ordinary_process_cannot_publish_a_service_identity_query_grant() {
    // SAFETY: the pseudo-handle is borrowed only while reading this process's descriptor.
    let process = unsafe { GetCurrentProcess() };
    let before = kernel_descriptor(process);
    let before_entries = ace_bytes(descriptor_dacl(&before));
    assert!(matches!(
        prepare_current_service(),
        Err(WindowsTransportError::ServerIdentityRejected)
    ));
    let after = kernel_descriptor(process);
    assert_eq!(ace_bytes(descriptor_dacl(&after)), before_entries);
}

fn descriptor(value: &str) -> LocalAllocation {
    let text: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = LocalAllocation::empty();
    // SAFETY: the terminated test SDDL and output slot are live; descriptor owns the result.
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor.0,
                null_mut(),
            )
        },
        0
    );
    descriptor
}

fn descriptor_dacl(descriptor: &LocalAllocation) -> *mut ACL {
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl = null_mut();
    // SAFETY: descriptor is a Windows-initialized security descriptor and all outputs are live.
    assert_ne!(
        unsafe { GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut dacl, &mut defaulted) },
        0
    );
    assert_ne!(present, 0);
    assert!(!dacl.is_null());
    dacl
}

fn descriptor_owner(descriptor: &LocalAllocation) -> *mut c_void {
    let mut owner = null_mut();
    let mut defaulted = 0;
    // SAFETY: descriptor remains allocated while the returned owner pointer is used.
    assert_ne!(
        unsafe { GetSecurityDescriptorOwner(descriptor.0, &mut owner, &mut defaulted) },
        0
    );
    owner
}

fn kernel_descriptor(object: HANDLE) -> LocalAllocation {
    let mut descriptor = LocalAllocation::empty();
    // SAFETY: the test retains object and descriptor owns the returned allocation.
    assert_eq!(
        unsafe {
            GetSecurityInfo(
                object,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION
                    | windows_sys::Win32::Security::OWNER_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descriptor.0,
            )
        },
        0
    );
    descriptor
}

fn ace_bytes(acl: *const ACL) -> Vec<Vec<u8>> {
    // SAFETY: acl comes from a retained Windows-created allocation.
    let count = unsafe { (*acl).AceCount };
    (0..count)
        .map(|index| {
            let mut ace = null_mut();
            // SAFETY: the bounded ACE index belongs to the retained ACL.
            assert_ne!(unsafe { GetAce(acl, u32::from(index), &mut ace) }, 0);
            // SAFETY: GetAce returns a complete header in the retained Windows ACL.
            let length = unsafe { ace.cast::<ACE_HEADER>().read_unaligned().AceSize } as usize;
            assert!((size_of::<ACE_HEADER>()..=4096).contains(&length));
            // SAFETY: this complete ACE remains in the allocation for the copy.
            unsafe { std::slice::from_raw_parts(ace.cast::<u8>(), length) }.to_vec()
        })
        .collect()
}

fn current_token() -> OwnedHandle {
    let mut original = null_mut();
    // SAFETY: the current process is borrowed and the output becomes an owned token handle.
    assert_ne!(
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_DUPLICATE,
                &mut original,
            )
        },
        0
    );
    OwnedHandle::new(original).unwrap()
}

fn restricted_interactive_caller() -> OwnedHandle {
    let original = current_token();
    let mut sid = LocalAllocation::empty();
    let text: Vec<u16> = INTERACTIVE_SID.encode_utf16().chain(Some(0)).collect();
    // SAFETY: fixed terminated SID and owned output allocation.
    assert_ne!(
        unsafe { ConvertStringSidToSidW(text.as_ptr(), &mut sid.0) },
        0
    );
    let restrict = SID_AND_ATTRIBUTES {
        Sid: sid.0,
        Attributes: 0,
    };
    let mut restricted = null_mut();
    // SAFETY: all inputs remain live. Only a new token is restricted; no thread impersonates it.
    assert_ne!(
        unsafe {
            CreateRestrictedToken(
                original.raw(),
                DISABLE_MAX_PRIVILEGE,
                0,
                null(),
                0,
                null(),
                1,
                &restrict,
                &mut restricted,
            )
        },
        0
    );
    let restricted = OwnedHandle::new(restricted).unwrap();
    let mut caller = null_mut();
    // SAFETY: the copied impersonation token is used only as an explicit AccessCheck argument.
    assert_ne!(
        unsafe {
            DuplicateTokenEx(
                restricted.raw(),
                TOKEN_QUERY,
                null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut caller,
            )
        },
        0
    );
    OwnedHandle::new(caller).unwrap()
}

fn allows(acl: *mut ACL, caller: &OwnedHandle, identity: IdentityObject, desired: u32) -> bool {
    let owner = descriptor(SERVICE_DACL);
    let system = descriptor_owner(&owner);
    let mut security = SECURITY_DESCRIPTOR::default();
    // SAFETY: the descriptor, owner SID and supplied ACL remain live throughout AccessCheck.
    unsafe {
        assert_ne!(
            InitializeSecurityDescriptor((&raw mut security).cast(), SECURITY_DESCRIPTOR_REVISION),
            0
        );
        assert_ne!(
            SetSecurityDescriptorOwner((&raw mut security).cast(), system, 0),
            0
        );
        assert_ne!(
            SetSecurityDescriptorGroup((&raw mut security).cast(), system, 0),
            0
        );
        assert_ne!(
            SetSecurityDescriptorDacl((&raw mut security).cast(), 1, acl, 0),
            0
        );
    }
    let mapping = GENERIC_MAPPING {
        GenericRead: identity.query_access(),
        GenericWrite: 0,
        GenericExecute: 0,
        GenericAll: match identity {
            IdentityObject::Process => PROCESS_ALL_ACCESS,
            IdentityObject::Token => TOKEN_ALL_ACCESS,
        },
    };
    let mut privileges = PRIVILEGE_SET::default();
    let mut privilege_bytes = size_of::<PRIVILEGE_SET>() as u32;
    let mut granted = 0;
    let mut allowed = 0;
    // SAFETY: every input/output is initialized, bounded and live. No caller state is modified.
    assert_ne!(
        unsafe {
            AccessCheck(
                (&raw mut security).cast(),
                caller.raw(),
                desired,
                &mapping,
                &mut privileges,
                &mut privilege_bytes,
                &mut granted,
                &mut allowed,
            )
        },
        0,
        "{}",
        io::Error::last_os_error()
    );
    allowed != 0 && granted & desired == desired
}
