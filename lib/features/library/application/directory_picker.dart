import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/directory_picker.dart";

final directoryPickerProvider = Provider<DirectoryPicker>((ref) {
  throw StateError("DirectoryPicker must be bound during application startup");
});
