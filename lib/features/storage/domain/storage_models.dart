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
    required this.catalogLiveBytes,
    required this.catalogReclaimableBytes,
    required this.catalogReclamation,
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
  final BigInt catalogLiveBytes;
  final BigInt catalogReclaimableBytes;
  final CatalogReclamationModel catalogReclamation;
  final bool requiresRestart;
  final List<RetiredPreviewRootModel> retiredPreviewRoots;
}

enum CatalogReclamationPhase {
  idle,
  queued,
  inspecting,
  waitingForIdle,
  checkingCapacity,
  converting,
  reclaiming,
  completed,
  cancelled,
  failed,
}

class CatalogReclamationModel {
  const CatalogReclamationModel({
    required this.operationId,
    required this.phase,
    required this.catalogFileBytes,
    required this.liveBytes,
    required this.reclaimableBytes,
    required this.reclaimedBytes,
    required this.requiredTemporaryBytes,
    required this.availableTemporaryBytes,
    required this.errorCode,
    required this.errorMessage,
  });

  final String? operationId;
  final CatalogReclamationPhase phase;
  final BigInt catalogFileBytes;
  final BigInt liveBytes;
  final BigInt reclaimableBytes;
  final BigInt reclaimedBytes;
  final BigInt? requiredTemporaryBytes;
  final BigInt? availableTemporaryBytes;
  final String? errorCode;
  final String? errorMessage;

  bool get isActive => switch (phase) {
    CatalogReclamationPhase.queued ||
    CatalogReclamationPhase.inspecting ||
    CatalogReclamationPhase.waitingForIdle ||
    CatalogReclamationPhase.checkingCapacity ||
    CatalogReclamationPhase.converting ||
    CatalogReclamationPhase.reclaiming => true,
    CatalogReclamationPhase.idle ||
    CatalogReclamationPhase.completed ||
    CatalogReclamationPhase.cancelled ||
    CatalogReclamationPhase.failed => false,
  };

  bool get canRetry =>
      phase == CatalogReclamationPhase.cancelled ||
      phase == CatalogReclamationPhase.failed ||
      (phase == CatalogReclamationPhase.idle && reclaimableBytes > BigInt.zero);
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
