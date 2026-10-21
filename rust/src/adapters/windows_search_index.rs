use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::domain::ScanError;
use crate::ports::search_index::{IndexedFileHint, SearchIndex};

#[path = "windows_search_index/ole_db.rs"]
mod ole_db;

pub(crate) struct WindowsSearchIndex;

impl SearchIndex for WindowsSearchIndex {
    fn read(
        &self,
        root_path: &str,
        cancelled: &AtomicBool,
        accept: &mut dyn FnMut(&[IndexedFileHint]) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        let started = Instant::now();
        let scope = normalized_index_path(root_path).replace('\'', "''");
        let query = format!(
            "SELECT TOP 1000000 System.ItemUrl, System.Size, System.DateModified \
             FROM SystemIndex WHERE SCOPE='file:{scope}'"
        );
        let _apartment = ole_db::Apartment::new().map_err(provider_error)?;
        let mut cursor = ole_db::Cursor::open(&query).map_err(provider_error)?;
        let mut count = 0;
        let mut page = Vec::with_capacity(256);
        loop {
            if cancelled.load(Ordering::Acquire) || started.elapsed() > Duration::from_secs(10) {
                return Err(ScanError::new(
                    "search_index_interrupted",
                    "The optional index query was interrupted",
                ));
            }
            let Some(row) = cursor.next().map_err(provider_error)? else {
                break;
            };
            count += 1;
            if let Some(relative_path) = relative_hint_path(root_path, &row.path) {
                let is_image_candidate =
                    relative_path.rsplit('.').next().is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "jpg"
                                | "jpeg"
                                | "jpe"
                                | "jfif"
                                | "png"
                                | "gif"
                                | "bmp"
                                | "webp"
                                | "tif"
                                | "tiff"
                                | "ico"
                                | "avif"
                                | "heic"
                                | "heif"
                        )
                    });
                page.push(IndexedFileHint {
                    relative_path,
                    is_image_candidate,
                    file_size: row.size,
                    modified_unix_ms: row.modified_ticks.and_then(|ticks| {
                        i64::try_from(ticks / 10_000)
                            .ok()?
                            .checked_sub(11_644_473_600_000)
                    }),
                });
            }
            if page.len() == 256 {
                accept(&page)?;
                page.clear();
            }
        }
        if count == 1_000_000 {
            return Err(ScanError::new(
                "search_index_truncated",
                "The optional index query reached its row limit",
            ));
        }
        if !page.is_empty() {
            accept(&page)?;
        }
        Ok(count)
    }
}

fn relative_hint_path(root: &str, path: &str) -> Option<String> {
    let root = normalized_index_path(root);
    let root = root.trim_end_matches('/');
    let path = normalized_index_path(path);
    let prefix = path.get(..root.len())?;
    if !prefix.eq_ignore_ascii_case(root) || path.as_bytes().get(root.len()) != Some(&b'/') {
        return None;
    }
    let relative = path.get(root.len() + 1..)?;
    if relative
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | "..") || part.contains([':', '\0']))
    {
        return None;
    }
    Some(relative.to_owned())
}

fn normalized_index_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    if let Some(unc) = path.strip_prefix("//?/UNC/") {
        return format!("//{unc}");
    }
    path.strip_prefix("//?/").unwrap_or(&path).to_owned()
}

fn provider_error(error: windows::core::Error) -> ScanError {
    ScanError::new(
        "search_index_unavailable",
        format!("Windows Search query failed ({:08x})", error.code().0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_paths_require_a_complete_root_boundary() {
        assert_eq!(
            relative_hint_path("C:\\图库", "c:\\图库\\相册\\照片.jpg"),
            Some("相册/照片.jpg".into())
        );
        assert_eq!(
            relative_hint_path(r"\\?\C:\图库", r"C:\图库\照片.jpg"),
            Some("照片.jpg".into())
        );
        assert_eq!(
            relative_hint_path(r"\\?\UNC\server\pictures", r"\\server\pictures\照片.jpg"),
            Some("照片.jpg".into())
        );
        assert_eq!(normalized_index_path(r"\\?\C:\图库"), "C:/图库");
        for path in [
            "C:\\图库外\\a.jpg",
            "C:\\图库\\..\\a.jpg",
            "C:\\图库\\a.jpg:stream",
            "D:\\图库\\a.jpg",
        ] {
            assert_eq!(relative_hint_path("C:\\图库", path), None);
        }
    }
}
