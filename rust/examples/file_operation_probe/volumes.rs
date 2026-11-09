use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use rust_lib_cedarflake_ame::file_operations::{
    FileAction, FileOperationControl, FileOperationIssueKind as Kind, FileOperationRecovery,
    FileOperationRequest, NativeFileOperations, WindowsFileOperations,
};

use super::{CONTENT, ProbeResult, request};

pub(super) fn cross_volume(engine: &WindowsFileOperations, source: &Path) -> ProbeResult {
    let destination = tempfile::Builder::new()
        .prefix("ame-r8-cross-volume-")
        .tempdir_in("R:\\")?
        .keep();
    let control = FileOperationControl::default();
    let mut request = request(
        engine,
        source,
        &destination,
        "cross-volume.txt",
        FileAction::Move,
    )?;
    assert_eq!(
        engine
            .execute(&request, &control)
            .expect_err("Handle rename must reject a different volume unchanged")
            .kind,
        Kind::DifferentVolume
    );
    assert_eq!(fs::read(source)?, CONTENT);
    assert!(!destination.join("cross-volume.txt").exists());
    request.action = FileAction::Copy;
    let copied = engine
        .execute(&request, &control)?
        .destination
        .ok_or("Missing copied destination")?;
    assert_eq!(fs::read(source)?, CONTENT);
    assert_eq!(fs::read(&copied.state.absolute_path)?, CONTENT);
    engine.complete_move(&request.source, &copied, &control)?;
    assert!(!source.exists());
    assert_eq!(fs::read(&copied.state.absolute_path)?, CONTENT);
    Ok("Cross-volume rename refused unchanged; native copy and verified source removal preserved the complete target".to_owned())
}

pub(super) fn recycle_fallback(engine: &WindowsFileOperations, scenario: &str) -> ProbeResult {
    let directory = tempfile::Builder::new()
        .prefix("ame-r8-recycle-fallback-")
        .tempdir_in("R:\\")?
        .keep();
    let source = directory.join("larger-than-recycle-quota.bin");
    let mut file = fs::File::create(&source)?;
    let block = vec![0x6b; 1024 * 1024];
    let mut expected = blake3::Hasher::new();
    for _ in 0..24 {
        file.write_all(&block)?;
        expected.update(&block);
    }
    file.sync_all()?;
    drop(file);
    let request = FileOperationRequest {
        action: FileAction::Recycle,
        source: engine.inspect_source(source.to_str().ok_or("Invalid fallback source")?)?,
        destination: None,
    };
    let result = engine.execute(&request, &FileOperationControl::default());
    if scenario == "fallback-refuse" {
        assert_eq!(
            result
                .expect_err("Refusing native fallback must cancel deletion")
                .kind,
            Kind::Cancelled
        );
        let mut original = fs::File::open(&source)?;
        let mut actual = blake3::Hasher::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let count = original.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            actual.update(&buffer[..count]);
        }
        assert_eq!(actual.finalize(), expected.finalize());
        Ok("Refusing non-recyclable fallback preserved all original bytes".to_owned())
    } else {
        assert_eq!(result?.recovery, FileOperationRecovery::None);
        assert!(!source.exists());
        Ok(
            "Explicit native fallback confirmation removed only the selected non-recyclable sample"
                .to_owned(),
        )
    }
}
