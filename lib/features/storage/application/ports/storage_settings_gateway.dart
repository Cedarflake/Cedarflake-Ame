import "../../domain/storage_models.dart";

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

  Future<CatalogReclamationModel> startCatalogReclamation({
    required String operationId,
  });

  Future<CatalogReclamationModel> loadCatalogReclamation();

  Future<bool> cancelCatalogReclamation({required String operationId});
}
