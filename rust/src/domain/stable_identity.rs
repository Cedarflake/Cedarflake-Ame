use blake3::Hasher;

pub(crate) fn stable_id(namespace: &str, value: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(namespace.as_bytes());
    hasher.update(&[0]);
    hasher.update(value.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_change_with_namespace() {
        assert_ne!(
            stable_id("library-root-v1", "C:\\pictures"),
            stable_id("asset-location-v1", "C:\\pictures"),
        );
    }
}
