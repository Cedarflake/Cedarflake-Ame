use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

static SOURCE_CONTENT_OPENS: LazyLock<SourceContentOpenCounts> =
    LazyLock::new(SourceContentOpenCounts::default);

#[derive(Default)]
struct SourceContentOpenCounts {
    roots: Mutex<HashMap<PathBuf, u64>>,
}

impl SourceContentOpenCounts {
    fn reset(&self, root: PathBuf) {
        self.roots
            .lock()
            .expect("source content open instrumentation")
            .insert(root, 0);
    }

    fn count(&self, root: &Path) -> u64 {
        self.roots
            .lock()
            .expect("source content open instrumentation")
            .get(root)
            .copied()
            .unwrap_or(0)
    }

    fn record(&self, resolve_root: impl FnOnce() -> PathBuf) {
        let is_inactive = self
            .roots
            .lock()
            .expect("source content open instrumentation")
            .is_empty();
        if is_inactive {
            return;
        }
        let root = resolve_root();
        let mut counts = self
            .roots
            .lock()
            .expect("source content open instrumentation");
        if let Some(count) = counts.get_mut(&root) {
            *count = count.saturating_add(1);
        }
    }
}

pub(crate) fn reset_source_content_open_instrumentation(root_path: &str) {
    let root =
        std::fs::canonicalize(root_path).expect("instrumented source root is canonicalizable");
    SOURCE_CONTENT_OPENS.reset(root);
}

pub(crate) fn source_content_open_count(root_path: &str) -> u64 {
    let root =
        std::fs::canonicalize(root_path).expect("instrumented source root is canonicalizable");
    SOURCE_CONTENT_OPENS.count(&root)
}

pub(super) fn record_source_content_open(root_path: &Path) {
    SOURCE_CONTENT_OPENS.record(|| {
        std::fs::canonicalize(root_path).expect("opened source root is canonicalizable")
    });
}

mod tests;
