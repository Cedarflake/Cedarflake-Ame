use crate::domain::ScanError;
use crate::domain::sidebar_order::{SidebarOrderKind, SidebarOrderPlacement};

pub fn reorder_sidebar_entry(
    kind: SidebarOrderKind,
    entry_id: String,
    anchor_id: String,
    placement: SidebarOrderPlacement,
) -> Result<(), ScanError> {
    crate::application::sidebar_order::reorder(kind, entry_id, anchor_id, placement)
}
