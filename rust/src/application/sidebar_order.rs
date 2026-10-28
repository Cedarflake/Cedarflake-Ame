use crate::domain::ScanError;
use crate::domain::sidebar_order::{SidebarOrderKind, SidebarOrderPlacement, validate_entry_id};
use crate::ports::sidebar_order::SidebarOrderRepository;

pub(crate) fn reorder(
    kind: SidebarOrderKind,
    entry_id: String,
    anchor_id: String,
    placement: SidebarOrderPlacement,
) -> Result<(), ScanError> {
    validate_entry_id(&entry_id)?;
    validate_entry_id(&anchor_id)?;
    super::organization::open_catalog()?
        .reorder_sidebar_entry(kind, &entry_id, &anchor_id, placement)
}
