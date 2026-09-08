use std::time::{Duration, Instant};

use super::SqliteCatalog;

impl SqliteCatalog {
    pub(crate) fn retire_with_poll_diagnostics(self) {
        // Match the implicit field destruction order. This test-only decomposition neither
        // retains connections nor moves their cleanup outside the measured poll.
        let Self {
            path,
            connection,
            _identity_guard: identity_guard,
            session,
            write_admission,
            pending_locations,
            pending_authoritative_retry_paths,
        } = self;
        let started = Instant::now();
        drop(path);
        let connection_started = Instant::now();
        // Includes rusqlite statement-cache/hook disposal and sqlite3_close, not only the FFI call.
        drop(connection);
        let connection_elapsed = connection_started.elapsed();
        let identity_started = Instant::now();
        let had_identity_guard = identity_guard.is_some();
        drop(identity_guard);
        let identity_elapsed = identity_started.elapsed();
        let session_started = Instant::now();
        drop(session);
        drop(write_admission);
        let session_elapsed = session_started.elapsed();
        let pending_started = Instant::now();
        drop(pending_locations);
        drop(pending_authoritative_retry_paths);
        let pending_elapsed = pending_started.elapsed();
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(100) {
            eprintln!(
                "[Ame sync catalog retirement] total_ms={} connection_drop_ms={} \
                 identity_guard_ms={} identity_guard_present={had_identity_guard} \
                 session_admission_ms={} pending_buffers_ms={}",
                elapsed.as_millis(),
                connection_elapsed.as_millis(),
                identity_elapsed.as_millis(),
                session_elapsed.as_millis(),
                pending_elapsed.as_millis(),
            );
        }
    }
}
