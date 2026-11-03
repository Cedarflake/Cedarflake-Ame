pub(super) fn forbidden_rule(
    namespace: &str,
    target: &str,
    is_module_reference: bool,
) -> Option<&'static str> {
    let layer = namespace.split("::").nth(1)?;
    if layer == "api" && target == "crate::frb_generated::StreamSink" {
        return None;
    }
    let target_layer = target
        .strip_prefix("crate::")
        .and_then(|path| path.split("::").next());
    let outward = match layer {
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
    if layer == "application" && target_layer == Some("adapters") {
        return Some("concrete-adapter");
    }
    if layer == "adapters" {
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
    if layer != "api" && external == "flutter_rust_bridge" {
        return Some("bridge-runtime");
    }
    if matches!(layer, "domain" | "ports")
        && ["std::fs", "std::process", "std::net"]
            .iter()
            .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
    {
        return Some("platform-io");
    }
    None
}
