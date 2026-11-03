pub(super) fn layer(namespace: &str) -> Option<&str> {
    if namespace.starts_with("binary::") {
        return Some("adapters");
    }
    if namespace == "crate" {
        return Some("application");
    }
    match namespace.strip_prefix("crate::")?.split("::").next()? {
        "journal_broker" | "windows_usn" | "classification_worker" => Some("adapters"),
        layer @ ("domain" | "ports" | "application" | "api" | "adapters" | "frb_generated") => {
            Some(layer)
        }
        _ => Some("application"),
    }
}

pub(super) fn forbidden_rule(
    namespace: &str,
    target: &str,
    is_module_reference: bool,
) -> Option<&'static str> {
    let source_layer = layer(namespace)?;
    if source_layer == "api" && target == "crate::frb_generated::StreamSink" {
        return None;
    }
    let target_layer = layer(target);
    let outward = match source_layer {
        "domain" => matches!(
            target_layer,
            Some("ports" | "application" | "adapters" | "api" | "frb_generated")
        ),
        "ports" => matches!(
            target_layer,
            Some("application" | "adapters" | "api" | "frb_generated")
        ),
        "application" => matches!(target_layer, Some("api" | "frb_generated")),
        "api" => matches!(target_layer, Some("adapters" | "frb_generated")),
        "adapters" => matches!(target_layer, Some("application" | "api" | "frb_generated")),
        _ => false,
    };
    if outward {
        return Some("outward-layer");
    }
    if source_layer == "application" && target_layer == Some("adapters") {
        return Some("concrete-adapter");
    }
    if source_layer == "adapters" {
        return None;
    }
    let external = target.split("::").next().unwrap_or_default();
    if (target.contains("::") || is_module_reference)
        && matches!(
            external,
            "rusqlite"
                | "image"
                | "exif"
                | "jpeg_decoder"
                | "zune_jpeg"
                | "zune_core"
                | "ort"
                | "tokenizers"
                | "reqwest"
                | "windows"
                | "windows_sys"
                | "notify"
                | "directories"
        )
    {
        return Some("external-engine");
    }
    if source_layer != "api" && external == "flutter_rust_bridge" {
        return Some("bridge-runtime");
    }
    if matches!(source_layer, "domain" | "ports")
        && ["std::fs", "std::process", "std::net"]
            .iter()
            .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
    {
        return Some("platform-io");
    }
    None
}
