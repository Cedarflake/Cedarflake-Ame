import "package:flutter_riverpod/misc.dart";

import "../../features/storage/adapters/platform/storage_directory_picker.dart";
import "../../features/storage/adapters/rust/rust_storage_settings_gateway.dart";
import "../../features/storage/application/ports/storage_directory_picker.dart";
import "../../features/storage/application/ports/storage_settings_gateway.dart";
import "../../features/storage/application/storage_settings.dart";

List<Override> storageBindings({
  StorageSettingsGateway gateway = const RustStorageSettingsGateway(),
  StorageDirectoryPicker directoryPicker =
      const PlatformStorageDirectoryPicker(),
}) => [
  storageSettingsGatewayProvider.overrideWithValue(gateway),
  storageDirectoryPickerProvider.overrideWithValue(directoryPicker),
];

final storageRequirements = <ProviderListenable<Object?>>[
  storageSettingsGatewayProvider,
  storageDirectoryPickerProvider,
];
