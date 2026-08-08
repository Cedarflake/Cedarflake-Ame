import "package:file_selector/file_selector.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/storage.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/storage_models.dart";

abstract interface class StorageSettingsGateway {
  Future<StorageStatusModel> load();

  Future<StorageStatusModel> update({
    String? catalogDirectory,
    String? previewCacheDirectory,
    required BigInt previewBudgetBytes,
  });
}

class RustStorageSettingsGateway implements StorageSettingsGateway {
  const RustStorageSettingsGateway();

  @override
  Future<StorageStatusModel> load() async {
    try {
      return _mapStatus(rust_api.loadStorageStatus());
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_storage_status_failed");
    }
  }

  @override
  Future<StorageStatusModel> update({
    String? catalogDirectory,
    String? previewCacheDirectory,
    required BigInt previewBudgetBytes,
  }) async {
    try {
      return _mapStatus(
        rust_api.updateStorageSettings(
          update: rust_domain.StorageSettingsUpdate(
            catalogDirectory: catalogDirectory,
            previewCacheDirectory: previewCacheDirectory,
            previewBudgetBytes: previewBudgetBytes,
          ),
        ),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_storage_update_failed");
    }
  }

  StorageStatusModel _mapStatus(rust_domain.StorageStatus status) {
    return StorageStatusModel(
      settingsPath: status.settingsPath,
      activeCatalogPath: status.activeCatalogPath,
      activePreviewRoot: status.activePreviewRoot,
      configuredCatalogPath: status.configuredCatalogPath,
      configuredPreviewRoot: status.configuredPreviewRoot,
      configuredCatalogDisplayPath: status.configuredCatalogDisplayPath,
      configuredPreviewDisplayPath: status.configuredPreviewDisplayPath,
      previewBudgetBytes: status.previewBudgetBytes,
      previewUsedBytes: status.previewUsedBytes,
      catalogUsedBytes: status.catalogUsedBytes,
      requiresRestart: status.requiresRestart,
    );
  }

  StorageSettingsFailure _mapFailure(Object error, String fallbackCode) {
    if (error case rust_domain.ScanError(:final code, :final message)) {
      return StorageSettingsFailure(code: code, message: message);
    }
    return StorageSettingsFailure(
      code: fallbackCode,
      message: error.toString(),
    );
  }
}

abstract interface class StorageDirectoryPicker {
  Future<String?> pick({required String initialDirectory});
}

class PlatformStorageDirectoryPicker implements StorageDirectoryPicker {
  const PlatformStorageDirectoryPicker();

  @override
  Future<String?> pick({required String initialDirectory}) {
    return getDirectoryPath(
      initialDirectory: initialDirectory,
      confirmButtonText: "选择此文件夹",
    );
  }
}

final storageSettingsGatewayProvider = Provider<StorageSettingsGateway>((ref) {
  return const RustStorageSettingsGateway();
});

final storageDirectoryPickerProvider = Provider<StorageDirectoryPicker>((ref) {
  return const PlatformStorageDirectoryPicker();
});
