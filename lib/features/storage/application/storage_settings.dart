import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/storage_directory_picker.dart";
import "ports/storage_settings_gateway.dart";

final storageSettingsGatewayProvider = Provider<StorageSettingsGateway>((ref) {
  throw StateError(
    "StorageSettingsGateway must be bound during application startup",
  );
});

final storageDirectoryPickerProvider = Provider<StorageDirectoryPicker>((ref) {
  throw StateError(
    "StorageDirectoryPicker must be bound during application startup",
  );
});
