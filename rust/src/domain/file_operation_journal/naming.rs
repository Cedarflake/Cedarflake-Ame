use std::path::Path;

pub fn numbered_file_name(original: &str, number: u32) -> String {
    if number == 0 {
        return original.to_owned();
    }
    let (stem, suffix) = parts(original);
    format!("{stem}({number}){suffix}")
}

pub(super) fn is_numbered_name(original: &str, actual: &str) -> bool {
    if original == actual {
        return true;
    }
    let (stem, suffix) = parts(original);
    actual
        .strip_prefix(&format!("{stem}("))
        .and_then(|value| value.strip_suffix(&format!("){suffix}")))
        .is_some_and(|number| {
            !number.starts_with('0')
                && number.bytes().all(|value| value.is_ascii_digit())
                && number.parse::<u32>().is_ok_and(|value| value > 0)
        })
}

fn parts(original: &str) -> (&str, String) {
    let path = Path::new(original);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(original);
    let suffix = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    (stem, suffix)
}
