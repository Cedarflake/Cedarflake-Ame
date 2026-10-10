import "../domain/storage_models.dart";

/// A settings save confirms configuration, not the age of accompanying usage.
class StorageConfigurationProjection {
  StorageConfigurationProjection.fromStatus(StorageStatusModel status)
    : configuredCatalogPath = status.configuredCatalogPath,
      configuredPreviewRoot = status.configuredPreviewRoot,
      configuredCatalogDisplayPath = status.configuredCatalogDisplayPath,
      configuredPreviewDisplayPath = status.configuredPreviewDisplayPath,
      previewBudgetBytes = status.previewBudgetBytes,
      requiresRestart = status.requiresRestart;

  final String configuredCatalogPath;
  final String configuredPreviewRoot;
  final String configuredCatalogDisplayPath;
  final String configuredPreviewDisplayPath;
  final BigInt previewBudgetBytes;
  final bool requiresRestart;

  StorageStatusModel applyTo(StorageStatusModel usage) => StorageStatusModel(
    settingsPath: usage.settingsPath,
    activeCatalogPath: usage.activeCatalogPath,
    activePreviewRoot: usage.activePreviewRoot,
    configuredCatalogPath: configuredCatalogPath,
    configuredPreviewRoot: configuredPreviewRoot,
    configuredCatalogDisplayPath: configuredCatalogDisplayPath,
    configuredPreviewDisplayPath: configuredPreviewDisplayPath,
    previewBudgetBytes: previewBudgetBytes,
    previewUsedBytes: usage.previewUsedBytes,
    catalogUsedBytes: usage.catalogUsedBytes,
    catalogLiveBytes: usage.catalogLiveBytes,
    catalogReclaimableBytes: usage.catalogReclaimableBytes,
    catalogReclamation: usage.catalogReclamation,
    requiresRestart: requiresRestart,
    retiredPreviewRoots: usage.retiredPreviewRoots,
  );
}
