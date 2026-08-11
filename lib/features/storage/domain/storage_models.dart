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
    required this.retiredPreviewRoots,
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
  final List<RetiredPreviewRootModel> retiredPreviewRoots;
}

class RetiredPreviewRootModel {
  const RetiredPreviewRootModel({
    required this.previewRoot,
    required this.displayPath,
  });

  final String previewRoot;
  final String displayPath;
}

class StorageSettingsFailure implements Exception {
  const StorageSettingsFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}

enum PreviewCleanupPhase { started, running, completed, cancelled, failed }

class PreviewCleanupUpdate {
  const PreviewCleanupUpdate({
    required this.operationId,
    required this.phase,
    required this.processedFiles,
    required this.totalFiles,
    required this.removedFiles,
    required this.removedBytes,
    required this.issueCount,
    this.issueMessage,
    this.errorMessage,
  });

  final String operationId;
  final PreviewCleanupPhase phase;
  final BigInt processedFiles;
  final BigInt totalFiles;
  final BigInt removedFiles;
  final BigInt removedBytes;
  final BigInt issueCount;
  final String? issueMessage;
  final String? errorMessage;

  bool get isActive =>
      phase == PreviewCleanupPhase.started ||
      phase == PreviewCleanupPhase.running;

  bool get isTerminal => !isActive;
}
