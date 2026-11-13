import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_catalog.dart";

const libraryCatalogWindow = 500;
const libraryTimelineWindow = 160;
const libraryFolderWindow = 200;

final libraryCatalogProvider = Provider<LibraryCatalog>((ref) {
  throw StateError("LibraryCatalog must be bound during application startup");
});

final libraryFolderCatalogProvider = Provider<LibraryFolderCatalog>((ref) {
  throw StateError(
    "LibraryFolderCatalog must be bound during application startup",
  );
});

final libraryGroupMemberCatalogProvider = Provider<LibraryGroupMemberCatalog>((
  ref,
) {
  throw StateError(
    "LibraryGroupMemberCatalog must be bound during application startup",
  );
});
