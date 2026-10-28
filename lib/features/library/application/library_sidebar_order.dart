import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/sidebar_order.dart" as api;
import "../../../src/rust/domain/sidebar_order.dart" as dto;

enum LibrarySidebarKind { albums, sourceFolders }

enum LibrarySidebarPlacement { before, after }

class LibrarySidebarMove {
  const LibrarySidebarMove({
    required this.entryId,
    required this.anchorId,
    required this.placement,
  });

  final String entryId;
  final String anchorId;
  final LibrarySidebarPlacement placement;
}

abstract interface class LibrarySidebarOrder {
  Future<void> move(LibrarySidebarKind kind, LibrarySidebarMove move);
}

class RustLibrarySidebarOrder implements LibrarySidebarOrder {
  const RustLibrarySidebarOrder();

  @override
  Future<void> move(LibrarySidebarKind kind, LibrarySidebarMove move) =>
      api.reorderSidebarEntry(
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
      );
}

final librarySidebarOrderProvider = Provider<LibrarySidebarOrder>(
  (ref) => const RustLibrarySidebarOrder(),
);
