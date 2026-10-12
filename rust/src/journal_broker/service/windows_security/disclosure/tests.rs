use super::*;

use windows_sys::Win32::Security::{
    DuplicateTokenEx, SecurityImpersonation, TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_QUERY,
    TokenImpersonation,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::super::OwnedToken;

mod directory_acl;

use directory_acl::DirectoryAcl;

#[test]
fn disclosure_probe_requires_every_ancestor_and_never_opens_content() {
    let mut ancestors = Vec::new();
    let mut candidates = Vec::new();
    let visible = probe_disclosable_scope_with(
        r"C:\Root",
        &RootRelativeScope::Relative("album/item.jpg".to_owned()),
        |path| {
            ancestors.push(path.to_owned());
            Ok(())
        },
        |path| {
            candidates.push(path.to_owned());
            Ok(CandidateProbe::Accessible)
        },
    )
    .expect("accessible candidate");
    assert!(visible);
    assert_eq!(ancestors, [r"C:\Root", r"C:\Root\album"]);
    assert_eq!(candidates, [r"C:\Root\album\item.jpg"]);

    assert!(
        probe_disclosable_scope_with(
            r"C:\Root",
            &RootRelativeScope::Relative("deleted.jpg".to_owned()),
            |_| Ok(()),
            |_| Ok(CandidateProbe::Missing),
        )
        .expect("enumerable parent proves deleted name disclosure")
    );
    assert!(
        probe_disclosable_scope_with(
            r"C:\Root",
            &RootRelativeScope::Relative("private/item.jpg".to_owned()),
            |path| {
                if path.ends_with("private") {
                    Err(WindowsSecurityError::RootRejected)
                } else {
                    Ok(())
                }
            },
            |_| Ok(CandidateProbe::Accessible),
        )
        .is_err()
    );
    assert!(
        probe_disclosable_scope_with(
            r"C:\Root",
            &RootRelativeScope::Relative("denied.jpg".to_owned()),
            |_| Ok(()),
            |_| Err(WindowsSecurityError::RootRejected),
        )
        .is_err()
    );
}

#[test]
fn attributes_without_listing_cannot_disclose_existing_or_deleted_names() {
    let directory = tempfile::tempdir().expect("disposable root");
    let private = directory.path().join("private");
    std::fs::create_dir(&private).expect("private directory");
    std::fs::write(private.join("present.jpg"), b"generated fixture").expect("fixture file");
    let (root, _held_root, _) =
        super::super::describe_client_root(directory.path(), "disclosure-root", 1)
            .expect("canonical directory identity");
    let root_utf16 = root.canonical_root_utf16;
    let root_path = String::from_utf16(&root_utf16).expect("UTF-16 root path");
    let normalized = normalize_backend_path(&root_utf16).expect("normalized root");
    let comparisons =
        super::super::query_component_semantics(&root_utf16).expect("root component semantics");
    let present = RootRelativeScope::Relative("private/present.jpg".to_owned());
    let deleted = RootRelativeScope::Relative("private/deleted.jpg".to_owned());

    for scope in [&present, &deleted] {
        assert!(
            probe_disclosable_scope(&root_path, &normalized, &comparisons, scope)
                .expect("ordinary directory permits name disclosure")
        );
    }
    let acl = DirectoryAcl::deny_listing(&private);
    let private_utf16 = private.to_str().unwrap().encode_utf16().collect::<Vec<_>>();
    assert!(open_directory_handle(&private_utf16).is_ok());
    for scope in [&present, &deleted] {
        assert!(probe_disclosable_scope(&root_path, &normalized, &comparisons, scope).is_err());
    }
    drop(acl);
    assert!(
        probe_disclosable_scope(&root_path, &normalized, &comparisons, &present)
            .expect("restored directory permissions")
    );
}

#[test]
fn root_disclosure_requires_listing_and_a_directory_object() {
    let directory = tempfile::tempdir().expect("disposable root");
    let path = directory.path().to_str().expect("UTF-8 fixture path");
    assert!(open_listable_directory(path).is_ok());
    let acl = DirectoryAcl::deny_listing(directory.path());
    assert!(open_listable_directory(path).is_err());
    drop(acl);

    let file = directory.path().join("regular-file.jpg");
    std::fs::write(&file, b"generated fixture").expect("regular file");
    assert!(
        open_directory_handle(&file.to_str().unwrap().encode_utf16().collect::<Vec<_>>()).is_ok()
    );
    assert!(open_listable_directory(file.to_str().unwrap()).is_err());
}

#[test]
fn held_directory_rechecks_revoked_listing_under_captured_impersonation() {
    let directory = tempfile::tempdir().expect("disposable root");
    let path = directory
        .path()
        .to_str()
        .expect("UTF-8 generated root")
        .encode_utf16()
        .collect::<Vec<_>>();
    let held_directory = open_directory_handle(&path).expect("held directory before revocation");
    let token = duplicate_current_impersonation_token();
    token
        .impersonate(|| listable_directory(&held_directory).map(drop))
        .expect("captured caller can list the held directory");

    let acl = DirectoryAcl::deny_listing(directory.path());
    token
        .impersonate(|| open_directory_handle(&path).map(drop))
        .expect("revocation still permits caller attribute access");
    assert!(matches!(
        token.impersonate(|| listable_directory(&held_directory).map(drop)),
        Err(WindowsSecurityError::RootRejected)
    ));
    drop(acl);

    token
        .impersonate(|| listable_directory(&held_directory).map(drop))
        .expect("same captured caller and held directory after permission restoration");
}

fn duplicate_current_impersonation_token() -> OwnedToken {
    let mut primary = null_mut();
    // SAFETY: GetCurrentProcess names this test process and primary is a live output slot. The
    // source token is queried and duplicated only, without changing its privileges or groups.
    assert_ne!(
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_DUPLICATE,
                &mut primary,
            )
        },
        0
    );
    let primary = OwnedToken::new(primary).expect("current primary token");
    let mut captured = null_mut();
    // SAFETY: primary remains live, captured is a live output, and the unique duplicate is an
    // impersonation token owned by this test until all permission probes have reverted.
    assert_ne!(
        unsafe {
            DuplicateTokenEx(
                primary.raw(),
                TOKEN_QUERY | TOKEN_IMPERSONATE,
                null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut captured,
            )
        },
        0
    );
    OwnedToken::new(captured).expect("captured impersonation token")
}
