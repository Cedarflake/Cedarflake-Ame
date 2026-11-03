import "../../../../src/rust/api/sidebar_order.dart" as api;
import "../../../../src/rust/domain/sidebar_order.dart" as dto;
import "../../domain/library_sidebar_order.dart";
import "../../application/ports/library_sidebar_order.dart";
import "library_organization_failure.dart";

class RustLibrarySidebarOrder implements LibrarySidebarOrder {
  const RustLibrarySidebarOrder();

  @override
  Future<void> move(LibrarySidebarKind kind, LibrarySidebarMove move) =>
      invokeOrganizationBridge(
        () => api.reorderSidebarEntry(
          kind: switch (kind) {
            LibrarySidebarKind.albums => dto.SidebarOrderKind.albums,
            LibrarySidebarKind.sourceFolders =>
              dto.SidebarOrderKind.sourceFolders,
          },
          entryId: move.entryId,
          anchorId: move.anchorId,
          placement: switch (move.placement) {
            LibrarySidebarPlacement.before => dto.SidebarOrderPlacement.before,
            LibrarySidebarPlacement.after => dto.SidebarOrderPlacement.after,
          },
        ),
      );
}
