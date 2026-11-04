import "../../domain/library_sidebar_order.dart";

abstract interface class LibrarySidebarOrder {
  Future<void> move(LibrarySidebarKind kind, LibrarySidebarMove move);
}
