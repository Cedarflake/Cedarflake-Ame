use super::ScanError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarOrderKind {
    Albums,
    SourceFolders,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarOrderPlacement {
    Before,
    After,
}

pub(crate) fn validate_entry_id(value: &str) -> Result<(), ScanError> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(ScanError::new(
            "sidebar_order_invalid",
            "Sidebar ordering requires current item identifiers",
        ));
    }
    Ok(())
}
