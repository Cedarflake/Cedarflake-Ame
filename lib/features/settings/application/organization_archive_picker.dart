import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/organization_archive_picker.dart";

final organizationArchivePickerProvider = Provider<OrganizationArchivePicker>((
  ref,
) {
  throw StateError(
    "OrganizationArchivePicker must be bound during application startup",
  );
});
