use super::*;

#[test]
#[ignore = "bounded generated metadata-only revalidation cost diagnostic"]
fn compare_metadata_revalidation_handle_cost() {
    let directory = tempdir().expect("generated source states");
    let states: Vec<_> = (0..64)
        .map(|index| {
            let path = directory.path().join(format!("source-{index}.bin"));
            fs::write(&path, b"generated source evidence").expect("source fixture");
            let metadata = path.metadata().expect("fixture metadata");
            let (file_identity, revision) = file_source_evidence(&path).expect("fixture identity");
            ExpectedFileState {
                absolute_path: path_text(&path),
                file_size: metadata.len(),
                modified_unix_ms: modified_unix_ms(&metadata),
                file_identity,
                source_revision: Some(revision),
            }
        })
        .collect();
    let mut separate = Duration::ZERO;
    let mut shared = Duration::ZERO;
    for round in 0..4 {
        for mode in [round % 2, 1 - round % 2] {
            let started = Instant::now();
            for _ in 0..16 {
                for expected in &states {
                    if mode == 0 {
                        let path = Path::new(&expected.absolute_path);
                        let metadata = path.symlink_metadata().expect("path metadata");
                        let entry =
                            checked_directory_entry_from_metadata(String::new(), path, metadata)
                                .expect("admitted file");
                        assert!(entry.metadata.is_file());
                        assert_eq!(entry.reparse_kind, ReparseKind::None);
                        assert_eq!(
                            entry.placeholder_state,
                            MetadataInventoryPlaceholderState::Available
                        );
                        let identity = file_identity(path).expect("separate identity handle");
                        let (_, revision) =
                            file_source_evidence(path).expect("separate revision handle");
                        revalidate_file_state_values(
                            expected,
                            &entry.metadata,
                            identity,
                            Some(revision),
                        )
                        .expect("separate-handle evidence");
                    } else {
                        revalidate_file_state(expected)
                            .expect("production shared-handle validation");
                    }
                }
            }
            match mode {
                0 => separate += started.elapsed(),
                _ => shared += started.elapsed(),
            }
        }
    }
    eprintln!(
        "AME_REVALIDATION_HANDLES items=4096 separate_us={} production_us={}",
        separate.as_micros(),
        shared.as_micros(),
    );
    for expected in states {
        assert_eq!(
            fs::read(expected.absolute_path).unwrap(),
            b"generated source evidence"
        );
    }
}
