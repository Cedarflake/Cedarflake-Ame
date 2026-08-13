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

  Stream<PreviewCleanupUpdate> clearPreviews({required String operationId});

  Stream<PreviewCleanupUpdate> clearRetiredPreviews({
    required String previewRoot,
    required String operationId,
  });

  Future<bool> cancelPreviewCleanup({required String operationId});
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

  @override
  Stream<PreviewCleanupUpdate> clearPreviews({required String operationId}) {
    return _cleanupUpdates(
      operationId: operationId,
      events: rust_api.clearPreviewCache(operationId: operationId),
    );
  }

  @override
  Stream<PreviewCleanupUpdate> clearRetiredPreviews({
    required String previewRoot,
    required String operationId,
  }) {
    return _cleanupUpdates(
      operationId: operationId,
      events: rust_api.clearRetiredPreviewCache(
        previewRoot: previewRoot,
        operationId: operationId,
      ),
    );
  }

  Stream<PreviewCleanupUpdate> _cleanupUpdates({
    required String operationId,
    required Stream<rust_domain.PreviewCleanupEvent> events,
  }) async* {
    var current = PreviewCleanupUpdate(
      operationId: operationId,
      phase: PreviewCleanupPhase.started,
      processedFiles: BigInt.zero,
      totalFiles: BigInt.zero,
      removedFiles: BigInt.zero,
      removedBytes: BigInt.zero,
      issueCount: BigInt.zero,
    );
    try {
      await for (final event in events) {
        current = _mapCleanupEvent(event, current);
        yield current;
      }
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_preview_cleanup_failed");
    }
  }

  @override
  Future<bool> cancelPreviewCleanup({required String operationId}) async {
    try {
      return rust_api.cancelPreviewCacheCleanup(operationId: operationId);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_preview_cleanup_cancel_failed");
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
      retiredPreviewRoots: [
        for (final root in status.retiredPreviewRoots)
          RetiredPreviewRootModel(
            previewRoot: root.previewRoot,
            displayPath: root.displayPath,
          ),
      ],
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

  PreviewCleanupUpdate _mapCleanupEvent(
    rust_domain.PreviewCleanupEvent event,
    PreviewCleanupUpdate current,
  ) {
    return switch (event) {
      rust_domain.PreviewCleanupEvent_Started(
        :final operationId,
        :final totalFiles,
      ) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.started,
          processedFiles: BigInt.zero,
          totalFiles: totalFiles,
          removedFiles: BigInt.zero,
          removedBytes: BigInt.zero,
          issueCount: BigInt.zero,
        ),
      rust_domain.PreviewCleanupEvent_Progress(
        :final operationId,
        :final processedFiles,
        :final removedFiles,
        :final removedBytes,
        :final issueCount,
        :final totalFiles,
      ) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.running,
          processedFiles: processedFiles,
          totalFiles: totalFiles,
          removedFiles: removedFiles,
          removedBytes: removedBytes,
          issueCount: issueCount,
        ),
      rust_domain.PreviewCleanupEvent_Issue(:final operationId, :final issue) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.running,
          processedFiles: current.processedFiles,
          totalFiles: current.totalFiles,
          removedFiles: current.removedFiles,
          removedBytes: current.removedBytes,
          issueCount: current.issueCount + BigInt.one,
          issueMessage: issue.message,
        ),
      rust_domain.PreviewCleanupEvent_Completed(
        :final operationId,
        :final removedFiles,
        :final removedBytes,
        :final issueCount,
      ) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.completed,
          processedFiles: current.totalFiles,
          totalFiles: current.totalFiles,
          removedFiles: removedFiles,
          removedBytes: removedBytes,
          issueCount: issueCount,
        ),
      rust_domain.PreviewCleanupEvent_Cancelled(
        :final operationId,
        :final removedFiles,
        :final removedBytes,
        :final issueCount,
      ) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.cancelled,
          processedFiles: current.processedFiles,
          totalFiles: current.totalFiles,
          removedFiles: removedFiles,
          removedBytes: removedBytes,
          issueCount: issueCount,
        ),
      rust_domain.PreviewCleanupEvent_Failed(
        :final operationId,
        :final message,
      ) =>
        PreviewCleanupUpdate(
          operationId: operationId,
          phase: PreviewCleanupPhase.failed,
          processedFiles: current.processedFiles,
          totalFiles: current.totalFiles,
          removedFiles: current.removedFiles,
          removedBytes: current.removedBytes,
          issueCount: current.issueCount,
          errorMessage: message,
        ),
    };
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
