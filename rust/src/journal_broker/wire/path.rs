use super::{MAX_PATH_DEPTH, MAX_PATH_UTF16_UNITS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "R2c-O must query comparison semantics before host admission"
    )
)]
pub(crate) enum NameComparisonSemantics {
    OrdinalCaseSensitive,
    WindowsOrdinalIgnoreCase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathComponent {
    display: String,
    utf16: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedWindowsPath {
    drive_key: u8,
    components: Vec<PathComponent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RootRelativeScope {
    Root,
    Relative(String),
}

pub(crate) fn validate_root_path(path: &[u16]) -> Result<NormalizedWindowsPath, WireError> {
    normalize_absolute_path(path)
}

pub(crate) fn normalize_backend_path(path: &[u16]) -> Result<NormalizedWindowsPath, WireError> {
    normalize_absolute_path(path)
}

pub(crate) fn scope_under_root(
    path: &NormalizedWindowsPath,
    root: &NormalizedWindowsPath,
    root_component_comparisons: &[NameComparisonSemantics],
) -> Result<Option<RootRelativeScope>, WireError> {
    if root_component_comparisons.len() != root.components.len() {
        return Err(WireError::NameComparisonUnavailable);
    }
    if path.drive_key != root.drive_key || path.components.len() < root.components.len() {
        return Ok(None);
    }
    for ((candidate, root_component), comparison) in path
        .components
        .iter()
        .zip(&root.components)
        .zip(root_component_comparisons)
    {
        if !component_equals(&candidate.utf16, &root_component.utf16, *comparison)? {
            return Ok(None);
        }
    }

    if path.components.len() == root.components.len() {
        return Ok(Some(RootRelativeScope::Root));
    }

    Ok(Some(RootRelativeScope::Relative(
        path.components[root.components.len()..]
            .iter()
            .map(|component| component.display.as_str())
            .collect::<Vec<_>>()
            .join("/"),
    )))
}

impl NormalizedWindowsPath {
    pub(crate) fn component_count(&self) -> usize {
        self.components.len()
    }
}

pub(crate) fn validate_relative_path(path: &str) -> Result<(), WireError> {
    if path.is_empty()
        || path.encode_utf16().count() > MAX_PATH_UTF16_UNITS
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains("//")
        || path.contains('\\')
    {
        return Err(WireError::InvalidValue("relative_path"));
    }

    let mut depth = 0_usize;
    for component in path.split('/') {
        validate_component(component)?;
        depth = depth.checked_add(1).ok_or(WireError::LengthOverflow)?;
        if depth > MAX_PATH_DEPTH {
            return Err(WireError::LengthExceeded);
        }
    }
    Ok(())
}

fn normalize_absolute_path(path: &[u16]) -> Result<NormalizedWindowsPath, WireError> {
    if path.is_empty() || path.len() > MAX_PATH_UTF16_UNITS {
        return Err(WireError::LengthExceeded);
    }
    let decoded = String::from_utf16(path).map_err(|_| WireError::InvalidUtf16)?;
    if decoded.contains('\0') || decoded.contains('/') {
        return Err(WireError::InvalidValue("absolute_path"));
    }

    let mut chars = decoded.chars();
    let drive = chars
        .next()
        .filter(char::is_ascii_alphabetic)
        .ok_or(WireError::InvalidValue("drive_designator"))?;
    if chars.next() != Some(':') || chars.next() != Some('\\') {
        return Err(WireError::InvalidValue("absolute_path"));
    }

    let remainder: String = chars.collect();
    if remainder.starts_with('\\') || remainder.ends_with('\\') {
        return Err(WireError::InvalidValue("absolute_path"));
    }

    let components = if remainder.is_empty() {
        Vec::new()
    } else {
        let raw: Vec<&str> = remainder.split('\\').collect();
        if raw.iter().any(|component| component.is_empty()) || raw.len() > MAX_PATH_DEPTH {
            return Err(WireError::InvalidValue("absolute_path"));
        }
        let mut components = Vec::with_capacity(raw.len());
        for component in raw {
            validate_component(component)?;
            components.push(PathComponent {
                display: component.to_owned(),
                utf16: component.encode_utf16().collect(),
            });
        }
        components
    };

    Ok(NormalizedWindowsPath {
        drive_key: drive.to_ascii_uppercase() as u8,
        components,
    })
}

fn component_equals(
    left: &[u16],
    right: &[u16],
    comparison: NameComparisonSemantics,
) -> Result<bool, WireError> {
    match comparison {
        NameComparisonSemantics::OrdinalCaseSensitive => Ok(left == right),
        NameComparisonSemantics::WindowsOrdinalIgnoreCase => {
            #[cfg(windows)]
            {
                crate::journal_broker::windows::name::ordinal_equals(left, right, true)
            }
            #[cfg(not(windows))]
            {
                let _ = (left, right);
                Err(WireError::NameComparisonUnavailable)
            }
        }
    }
}

fn validate_component(component: &str) -> Result<(), WireError> {
    if component.is_empty()
        || matches!(component, "." | "..")
        || component.ends_with([' ', '.'])
        || component.chars().any(|character| {
            character < ' ' || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
        })
        || is_reserved_device_name(component)
    {
        return Err(WireError::InvalidValue("path_component"));
    }
    Ok(())
}

fn is_reserved_device_name(component: &str) -> bool {
    let base = component.split('.').next().unwrap_or(component);
    if ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|reserved| base.eq_ignore_ascii_case(reserved))
    {
        return true;
    }
    let bytes = base.as_bytes();
    bytes.len() == 4
        && ((bytes[0].eq_ignore_ascii_case(&b'C')
            && bytes[1].eq_ignore_ascii_case(&b'O')
            && bytes[2].eq_ignore_ascii_case(&b'M'))
            || (bytes[0].eq_ignore_ascii_case(&b'L')
                && bytes[1].eq_ignore_ascii_case(&b'P')
                && bytes[2].eq_ignore_ascii_case(&b'T')))
        && matches!(bytes[3], b'1'..=b'9')
}
