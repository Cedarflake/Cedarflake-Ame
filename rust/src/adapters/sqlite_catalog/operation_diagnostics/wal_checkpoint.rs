use std::cell::Cell;
use std::io::Write;
use std::time::{Duration, Instant};

use rusqlite::{Connection, hooks::Wal};

use super::super::connection_policy::{
    AUTOMATIC_CHECKPOINT_PAGES, INITIAL_IMPORT_CHECKPOINT_PAGES,
};

thread_local! {
    static CONFIGURED_PAGES: Cell<Option<i32>> = const { Cell::new(None) };
}

pub(in crate::adapters::sqlite_catalog) fn last_configured_checkpoint_pages() -> Option<i32> {
    CONFIGURED_PAGES.get()
}

pub(in crate::adapters::sqlite_catalog) fn observe_checkpoint(
    connection: &Connection,
) -> rusqlite::Result<()> {
    let frames: i32 = connection.query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))?;
    // Installing the diagnostic hook makes subsequent PRAGMA reads report zero.
    CONFIGURED_PAGES.set(Some(frames));
    match frames {
        AUTOMATIC_CHECKPOINT_PAGES => connection.wal_hook(Some(checkpoint)),
        INITIAL_IMPORT_CHECKPOINT_PAGES => connection.wal_hook(Some(initial_import_checkpoint)),
        _ => {}
    }
    Ok(())
}

fn checkpoint(wal: &Wal, frames: i32) -> rusqlite::Result<()> {
    checkpoint_at(wal, frames, AUTOMATIC_CHECKPOINT_PAGES)
}

fn initial_import_checkpoint(wal: &Wal, frames: i32) -> rusqlite::Result<()> {
    checkpoint_at(wal, frames, INITIAL_IMPORT_CHECKPOINT_PAGES)
}

fn checkpoint_at(wal: &Wal, frames: i32, threshold: i32) -> rusqlite::Result<()> {
    if frames >= threshold {
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
