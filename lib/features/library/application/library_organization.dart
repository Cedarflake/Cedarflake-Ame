import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_organization.dart";

final libraryOrganizationProvider = Provider<LibraryOrganization>((ref) {
  throw StateError(
    "LibraryOrganization must be bound during application startup",
  );
});
