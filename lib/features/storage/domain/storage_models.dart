class StorageStatusModel {
  const StorageStatusModel({
    required this.settingsPath,
    required this.activeCatalogPath,
    required this.activePreviewRoot,
    required this.configuredCatalogPath,
    required this.configuredPreviewRoot,
    required this.configuredCatalogDisplayPath,
    required this.configuredPreviewDisplayPath,
    required this.previewBudgetBytes,
    required this.previewUsedBytes,
    required this.catalogUsedBytes,
    required this.requiresRestart,
  });

  final String settingsPath;
  final String activeCatalogPath;
  final String activePreviewRoot;
  final String configuredCatalogPath;
  final String configuredPreviewRoot;
  final String configuredCatalogDisplayPath;
  final String configuredPreviewDisplayPath;
  final BigInt previewBudgetBytes;
  final BigInt previewUsedBytes;
  final BigInt catalogUsedBytes;
  final bool requiresRestart;
}

class StorageSettingsFailure implements Exception {
  const StorageSettingsFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
