use std::io::Write;
use std::time::{Duration, Instant};

use super::*;

pub(super) fn read(path: &Path) -> io::Result<CatalogFileIdentity> {
    let started = Instant::now();
    let file = open_validation_handle(path)?;
    let opened = Instant::now();
    let canonical_path = final_path_from_handle(&file);
    let resolved = Instant::now();
    let file_identity = canonical_path
        .is_ok()
        .then(|| file_identity_from_handle(&file));
    let identified = Instant::now();
    drop(file);
    let closed = Instant::now();
    if closed.duration_since(started) >= Duration::from_millis(25) {
        let _ = writeln!(
            std::io::stderr().lock(),
            "[Ame catalog identity] open_us={} path_us={} id_us={} close_us={} thread={:?}",
            opened.duration_since(started).as_micros(),
            resolved.duration_since(opened).as_micros(),
            identified.duration_since(resolved).as_micros(),
            closed.duration_since(identified).as_micros(),
            std::thread::current().id(),
        );
    }
    let canonical_path = canonical_path?;
    let file_identity = file_identity.expect("successful path query reached identity query")?;
    Ok(CatalogFileIdentity {
        canonical_path,
        file_identity,
    })
}
