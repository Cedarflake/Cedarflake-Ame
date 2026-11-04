pub(crate) fn user_visible_path(path: &str) -> String {
    const DEVICE_PREFIX: &str = "\\\\?\\";
    const UNC_DEVICE_PREFIX: &str = "\\\\?\\UNC\\";
    if path
        .get(..UNC_DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(UNC_DEVICE_PREFIX))
    {
        return format!("\\\\{}", &path[UNC_DEVICE_PREFIX.len()..]);
    }
    if path
        .get(..DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(DEVICE_PREFIX))
    {
        return path[DEVICE_PREFIX.len()..].to_owned();
    }
    path.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_visible_path_hides_windows_device_prefixes() {
        assert_eq!(
            user_visible_path(r"\\?\G:\Pictures\sample.png"),
            r"G:\Pictures\sample.png"
        );
        assert_eq!(
            user_visible_path(r"\\?\UNC\server\share\sample.png"),
            r"\\server\share\sample.png"
        );
        assert_eq!(
            user_visible_path(r"C:\Pictures\sample.png"),
            r"C:\Pictures\sample.png"
        );
    }
}
