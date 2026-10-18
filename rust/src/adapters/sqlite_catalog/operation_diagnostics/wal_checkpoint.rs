use std::io::Write;
use std::time::{Duration, Instant};

use rusqlite::{Connection, hooks::Wal};

const DEFAULT_CHECKPOINT_FRAMES: i32 = 1_000;

pub(in crate::adapters::sqlite_catalog) fn observe_default_checkpoint(
    connection: &Connection,
) -> rusqlite::Result<()> {
    let frames: i32 = connection.query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))?;
    if frames == DEFAULT_CHECKPOINT_FRAMES {
        connection.wal_hook(Some(checkpoint));
    }
    Ok(())
}

fn checkpoint(wal: &Wal, frames: i32) -> rusqlite::Result<()> {
    if frames >= DEFAULT_CHECKPOINT_FRAMES {
        let started = Instant::now();
        // SQLite's default hook uses the same passive checkpoint and ignores its result.
        let result = wal.checkpoint();
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(100) {
            let _ = writeln!(
                std::io::stderr().lock(),
                "[Ame automatic checkpoint] elapsed_ms={} frames={} result={:?} thread={:?}",
                elapsed.as_millis(),
                frames,
                result,
                std::thread::current().id(),
            );
        }
    }
    Ok(())
}
