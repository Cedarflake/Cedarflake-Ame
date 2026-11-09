use std::fs;
#[path = "volumes.rs"]
mod volumes;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rust_lib_cedarflake_ame::file_operations::{
    FileAction, FileOperationControl, FileOperationDestination, FileOperationIssueKind as Kind,
    FileOperationRecovery, FileOperationRequest, NativeFileOperations, WindowsFileOperations,
};

type ProbeResult = Result<String, Box<dyn std::error::Error>>;
const CONTENT: &[u8] = b"Ame operation adapter fixture: preserve these bytes";

pub(super) fn run(scenario: &str, root: &Path) -> ProbeResult {
    let engine = WindowsFileOperations::new(None);
    let source = root.join("图片.txt");
    let destination = root.join("destination");
    fs::create_dir(&destination)?;
    fs::write(&source, CONTENT)?;
    fs::write(root.join("untouched.txt"), b"outside operation selection")?;
    let receipt = match scenario {
        "transfers" => transfers(&engine, &source, &destination),
        "cancel-transfer" => cancel_transfer(&engine, &source, &destination),
        "complete-move" => complete_move(&engine, &source, &destination),
        "cross-volume" => volumes::cross_volume(&engine, &source),
        "fallback-refuse" | "fallback-accept" => volumes::recycle_fallback(&engine, scenario),
        "recycle" | "permanent" | "refuse-permanent" => deletion(&engine, &source, scenario),
        _ => Err("Unknown scenario".into()),
    }?;
    assert_eq!(
        fs::read(root.join("untouched.txt"))?,
        b"outside operation selection"
    );
    Ok(receipt)
}

fn request(
    engine: &WindowsFileOperations,
    source: &Path,
    destination: &Path,
    name: &str,
    action: FileAction,
) -> Result<FileOperationRequest, Box<dyn std::error::Error>> {
    Ok(FileOperationRequest {
        action,
        source: engine.inspect_source(source.to_str().ok_or("Invalid source encoding")?)?,
        destination: Some(FileOperationDestination {
            directory: engine
                .inspect_directory(destination.to_str().ok_or("Invalid target encoding")?)?,
            name: name.to_owned(),
        }),
    })
}

fn transfers(engine: &WindowsFileOperations, source: &Path, destination: &Path) -> ProbeResult {
    let control = FileOperationControl::default();
    let copy = request(engine, source, destination, "图片(1).txt", FileAction::Copy)?;
    let receipt = engine.execute(&copy, &control)?;
    let copied = destination.join("图片(1).txt");
    assert_eq!(fs::read(&copied)?, CONTENT);
    assert_eq!(fs::read(source)?, CONTENT);
    assert_eq!(
        receipt
            .destination
            .as_ref()
            .map(|file| file.state.file_size),
        Some(CONTENT.len() as u64)
    );
    let duplicate = engine
        .execute(&copy, &control)
        .expect_err("Existing destination must not be overwritten");
    assert_eq!(duplicate.kind, Kind::Collision);
    assert_eq!(fs::read(&copied)?, CONTENT);
    let rename = request(
        engine,
        &copied,
        destination,
        "renamed.txt",
        FileAction::Rename,
    )?;
    let receipt = engine.execute(&rename, &control)?;
    assert!(!copied.exists());
    let renamed = destination.join("renamed.txt");
    assert_eq!(fs::read(&renamed)?, CONTENT);
    assert_eq!(
        receipt
            .destination
            .as_ref()
            .and_then(|file| file.state.file_identity.as_ref()),
        rename.source.state.file_identity.as_ref()
    );
    let moved = request(
        engine,
        &renamed,
        source.parent().ok_or("Missing source parent")?,
        "moved.txt",
        FileAction::Move,
    )?;
    engine.execute(&moved, &control)?;
    assert!(!renamed.exists());
    assert_eq!(
        fs::read(source.parent().ok_or("Missing parent")?.join("moved.txt"))?,
        CONTENT
    );
    let cancelled = request(
        engine,
        source,
        destination,
        "cancelled.txt",
        FileAction::Copy,
    )?;
    let stop = FileOperationControl::default();
    stop.cancel();
    assert_eq!(
        engine
            .execute(&cancelled, &stop)
            .expect_err("Cancellation must prevent submission")
            .kind,
        Kind::Cancelled
    );
    assert!(!destination.join("cancelled.txt").exists());
    let mut invalid = request(
        engine,
        source,
        destination,
        "..\\escape.txt",
        FileAction::Copy,
    )?;
    assert_eq!(
        engine
            .execute(&invalid, &control)
            .expect_err("Escaping name must be rejected")
            .kind,
        Kind::InvalidRequest
    );
    invalid
        .destination
        .as_mut()
        .ok_or("Missing destination")?
        .name = "stale.txt".to_owned();
    fs::write(source, b"new content after authorization")?;
    assert_eq!(
        engine
            .execute(&invalid, &control)
            .expect_err("Stale source must be rejected")
            .kind,
        Kind::SourceChanged
    );
    assert!(!destination.join("stale.txt").exists());
    assert_eq!(fs::read(source)?, b"new content after authorization");
    Ok("Copy, no-overwrite collision, rename, move, cancellation, path escape and stale source passed".to_owned())
}

fn cancel_transfer(
    engine: &WindowsFileOperations,
    source: &Path,
    destination: &Path,
) -> ProbeResult {
    use std::io::{Read, Write};
    let mut file = fs::File::create(source)?;
    let block = vec![0x37; 1024 * 1024];
    let mut expected = blake3::Hasher::new();
    for _ in 0..64 {
        file.write_all(&block)?;
        expected.update(&block);
    }
    file.sync_all()?;
    drop(file);
    let operation = request(
        engine,
        source,
        destination,
        "cancelled.bin",
        FileAction::Copy,
    )?;
    let control = FileOperationControl::default();
    let finished = AtomicBool::new(false);
    let started = Instant::now();
    let result = std::thread::scope(|threads| {
        let observer = threads.spawn(|| {
            while !finished.load(Ordering::Acquire) && started.elapsed() < Duration::from_secs(15) {
                let (copied, total) = control.copy_progress();
                if copied > 0 && copied < total {
                    control.cancel();
                    return Some((copied, started.elapsed()));
                }
                std::thread::yield_now();
            }
            None
        });
        let outcome = engine.execute(&operation, &control);
        finished.store(true, Ordering::Release);
        (outcome, observer.join())
    });
    let requested = result
        .1
        .map_err(|_| "Cancellation observer failed")?
        .ok_or("No active-transfer cancellation was observed")?;
    assert_eq!(
        result.0.expect_err("The active copy must stop").kind,
        Kind::Cancelled
    );
    assert!(!destination.join("cancelled.bin").exists());
    assert_eq!(fs::metadata(source)?.len(), 64 * 1024 * 1024);
    let mut actual = blake3::Hasher::new();
    let mut file = fs::File::open(source)?;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        actual.update(&buffer[..count]);
    }
    assert_eq!(actual.finalize(), expected.finalize());
    Ok(format!(
        "Active copy cancelled after {} bytes; request {:?}; verified {:?}; source retained and partial target absent",
        requested.0,
        requested.1,
        started.elapsed()
    ))
}

fn complete_move(engine: &WindowsFileOperations, source: &Path, destination: &Path) -> ProbeResult {
    let control = FileOperationControl::default();
    let request = request(
        engine,
        source,
        destination,
        "verified.txt",
        FileAction::Copy,
    )?;
    let target = engine
        .execute(&request, &control)?
        .destination
        .ok_or("Copy returned no destination")?;
    let path = Path::new(&target.state.absolute_path);
    fs::write(path, vec![b'x'; CONTENT.len()])?;
    let changed = engine.inspect_source(&target.state.absolute_path)?;
    assert_eq!(
        engine
            .complete_move(&request.source, &changed, &control)
            .expect_err("Different bytes must retain the original")
            .kind,
        Kind::DestinationChanged
    );
    assert_eq!(fs::read(source)?, CONTENT);
    fs::write(path, CONTENT)?;
    let target = engine.inspect_source(&target.state.absolute_path)?;
    engine.complete_move(&request.source, &target, &control)?;
    assert!(!source.exists());
    assert_eq!(fs::read(path)?, CONTENT);
    Ok("Move completion rejected different bytes and removed only the original after matching verification".to_owned())
}

fn deletion(engine: &WindowsFileOperations, source: &Path, scenario: &str) -> ProbeResult {
    let action = if scenario == "recycle" {
        FileAction::Recycle
    } else {
        FileAction::DeletePermanently
    };
    let request = FileOperationRequest {
        action,
        source: engine.inspect_source(source.to_str().ok_or("Invalid source encoding")?)?,
        destination: None,
    };
    let result = engine.execute(&request, &FileOperationControl::default());
    if scenario == "refuse-permanent" {
        assert_eq!(
            result.expect_err("Declining must cancel deletion").kind,
            Kind::Cancelled
        );
        assert_eq!(fs::read(source)?, CONTENT);
        Ok("System refusal preserved the selected file".to_owned())
    } else {
        let receipt = result?;
        assert!(!source.exists());
        let recovery = if scenario == "recycle" {
            FileOperationRecovery::SystemRecycleBin
        } else {
            FileOperationRecovery::None
        };
        assert_eq!(receipt.recovery, recovery);
        Ok(format!(
            "Deleted exact selected file; recovery={recovery:?}"
        ))
    }
}
