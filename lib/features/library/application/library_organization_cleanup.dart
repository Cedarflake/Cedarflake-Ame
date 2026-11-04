import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_organization_cleanup.dart";

final libraryOrganizationCleanupProvider = Provider<LibraryOrganizationCleanup>(
  (ref) => throw StateError(
    "LibraryOrganizationCleanup must be bound during application startup",
  ),
);
