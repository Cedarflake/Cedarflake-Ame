import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_organization_batch.dart";

final libraryOrganizationBatchesProvider = Provider<LibraryOrganizationBatches>(
  (ref) {
    throw StateError(
      "LibraryOrganizationBatches must be bound during application startup",
    );
  },
);
