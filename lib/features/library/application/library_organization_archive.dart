import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_organization_archive.dart";

final libraryOrganizationArchivesProvider =
    Provider<LibraryOrganizationArchives>(
      (ref) => throw StateError(
        "LibraryOrganizationArchives must be bound during application startup",
      ),
    );
