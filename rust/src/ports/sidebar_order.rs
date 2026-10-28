use crate::domain::ScanError;
use crate::domain::sidebar_order::{SidebarOrderKind, SidebarOrderPlacement};

pub(crate) trait SidebarOrderRepository {
    fn reorder_sidebar_entry(
        &mut self,
        kind: SidebarOrderKind,
        entry_id: &str,
        anchor_id: &str,
        placement: SidebarOrderPlacement,
    ) -> Result<(), ScanError>;
}
