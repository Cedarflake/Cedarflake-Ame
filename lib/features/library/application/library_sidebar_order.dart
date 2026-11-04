import "package:flutter_riverpod/flutter_riverpod.dart";
import "ports/library_sidebar_order.dart";

final librarySidebarOrderProvider = Provider<LibrarySidebarOrder>(
  (ref) => throw StateError(
    "LibrarySidebarOrder must be bound during application startup",
  ),
);
