use std::collections::BTreeSet;

use windows_sys::Win32::System::Ioctl::{
    USN_REASON_FILE_DELETE, USN_REASON_RENAME_NEW_NAME, USN_REASON_RENAME_OLD_NAME,
};

use super::{FileReference, ParsedUsnRecord, reference_histories, resolve_reference_path_with};

#[test]
fn later_parent_delete_reconstructs_an_earlier_child_delete() {
    let records = vec![directory_record(2, 1, 31, "album", USN_REASON_FILE_DELETE)];

    assert_eq!(resolve_parent(2, &records), "library/album");
}

#[test]
fn nested_later_directory_deletes_reconstruct_the_complete_parent_chain() {
    let records = vec![
        directory_record(3, 2, 31, "inner", USN_REASON_FILE_DELETE),
        directory_record(2, 1, 32, "outer", USN_REASON_FILE_DELETE),
    ];

    assert_eq!(resolve_parent(3, &records), "library/outer/inner");
}

#[test]
fn later_parent_rename_uses_the_old_name_for_an_earlier_child() {
    let records = vec![
        directory_record(2, 1, 31, "old-album", USN_REASON_RENAME_OLD_NAME),
        directory_record(2, 1, 32, "new-album", USN_REASON_RENAME_NEW_NAME),
    ];

    assert_eq!(resolve_parent(2, &records), "library/old-album");
}

fn resolve_parent(reference: u64, records: &[ParsedUsnRecord]) -> String {
    resolve_reference_path_with(
        FileReference::V2(reference.to_le_bytes()),
        30,
        records,
        &reference_histories(records),
        &mut BTreeSet::new(),
        &|reference| {
            (reference == FileReference::V2(1_u64.to_le_bytes()))
                .then(|| "library".to_owned())
                .ok_or("Unexpected live reference lookup")
        },
        &|parent, name| Ok(format!("{parent}/{name}")),
    )
    .expect("historical parent path")
}

fn directory_record(
    reference: u64,
    parent: u64,
    usn: i64,
    name: &str,
    reason: u32,
) -> ParsedUsnRecord {
    ParsedUsnRecord {
        file_reference: FileReference::V2(reference.to_le_bytes()),
        parent_reference: FileReference::V2(parent.to_le_bytes()),
        usn,
        timestamp_100ns: 0,
        reason,
        file_attributes: 0x10,
        name: name.to_owned(),
    }
}
