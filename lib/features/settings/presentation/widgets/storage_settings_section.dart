import "dart:async";

import "package:flutter/foundation.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_choice.dart";
import "../../../storage/application/ports/storage_settings_gateway.dart";
import "../../../storage/application/storage_settings.dart";
import "../../../storage/domain/storage_models.dart";
import "../../../storage/application/catalog_reclamation_controller.dart";
import "settings_section.dart";
import "storage_preview_cleanup_confirmation.dart";
import "storage_settings_error.dart";

enum _StorageErrorTarget { settings, catalogReclamation, previewCleanup }

class StorageSettingsSection extends ConsumerStatefulWidget {
  const StorageSettingsSection({
    required this.hasLibraryRoots,
    this.libraryRootIds = const <String>{},
    super.key,
  });

  final bool hasLibraryRoots;
  final Set<String> libraryRootIds;

  @override
  ConsumerState<StorageSettingsSection> createState() =>
      _StorageSettingsSectionState();
}

class _StorageSettingsSectionState
    extends ConsumerState<StorageSettingsSection> {
  late final StorageSettingsGateway _storageGateway;
  late final CatalogReclamationController _reclamationController;
  PreviewCleanupUpdate? _cleanupUpdate;
  StreamSubscription<PreviewCleanupUpdate>? _cleanupSubscription;
  String? _cleanupTargetPreviewRoot;
  String? _cleanupTargetDisplayPath;
  Object? _settingsError;
  String? _previewCleanupErrorMessage;
  bool _isSaving = false;
  bool _isCancellingCleanup = false;
  int _storageLoadGeneration = 0;

  CatalogReclamationViewState get _reclamation => _reclamationController.state;
  StorageStatusModel? get _status => _reclamation.storageStatus;

  @override
  void initState() {
    super.initState();
    _storageGateway = ref.read(storageSettingsGatewayProvider);
    _reclamationController = CatalogReclamationController(
      gateway: _storageGateway,
    )..addListener(_handleReclamationChanged);
    unawaited(_load());
  }

  @override
  void didUpdateWidget(covariant StorageSettingsSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!setEquals(oldWidget.libraryRootIds, widget.libraryRootIds)) {
      unawaited(_load(errorTarget: _StorageErrorTarget.catalogReclamation));
    }
  }

  @override
  void dispose() {
    final cleanup = _cleanupUpdate;
    _reclamationController.dispose();
    unawaited(_cleanupSubscription?.cancel());
    if (cleanup != null && cleanup.isActive) {
      unawaited(
        _storageGateway.cancelPreviewCleanup(operationId: cleanup.operationId),
      );
    }
    super.dispose();
  }

  Future<bool> _load({
    _StorageErrorTarget errorTarget = _StorageErrorTarget.settings,
  }) async {
    final effectiveTarget = _status == null
        ? _StorageErrorTarget.settings
        : errorTarget;
    final generation = ++_storageLoadGeneration;
    final reclamationRequest = _reclamationController.beginStorageSnapshot();
    setState(() => _setError(effectiveTarget, null));
    try {
      final status = await ref.read(storageSettingsGatewayProvider).load();
      if (!mounted || generation != _storageLoadGeneration) {
        return false;
      }
      return _applyStatus(
        status,
        reclamationRequest: reclamationRequest,
        clearError: effectiveTarget,
      );
    } on Object catch (error) {
      if (mounted && generation == _storageLoadGeneration) {
        if (effectiveTarget == _StorageErrorTarget.catalogReclamation) {
          _reclamationController.failStorageSnapshot(reclamationRequest, error);
        } else {
          setState(() => _setError(effectiveTarget, error));
        }
      }
      return false;
    }
  }

  bool _applyStatus(
    StorageStatusModel status, {
    required CatalogReclamationSnapshotRequest reclamationRequest,
    _StorageErrorTarget? clearError,
  }) {
    if (!_reclamationController.acceptStorageSnapshot(
      reclamationRequest,
      status,
    )) {
      return false;
    }
    setState(() {
      if (clearError != null) {
        _setError(clearError, null);
      }
    });
    return true;
  }

  void _handleReclamationChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _chooseCatalogDirectory() async {
    final status = _status;
    if (status == null || widget.hasLibraryRoots || _isSaving) {
      return;
    }
    final directory = await ref
        .read(storageDirectoryPickerProvider)
        .pick(currentPath: status.configuredCatalogPath);
    if (directory != null && mounted) {
      await _update(catalogDirectory: directory);
    }
  }

  Future<void> _choosePreviewDirectory() async {
    final status = _status;
    if (status == null || _isSaving) {
      return;
    }
    final directory = await ref
        .read(storageDirectoryPickerProvider)
        .pick(currentPath: status.configuredPreviewRoot);
    if (directory != null && mounted) {
      await _update(previewDirectory: directory);
    }
  }

  Future<void> _update({
    String? catalogDirectory,
    String? previewDirectory,
    BigInt? previewBudgetBytes,
  }) async {
    final status = _status;
    if (status == null || _isSaving) {
      return;
    }
    setState(() {
      _isSaving = true;
      _settingsError = null;
    });
    final configurationRequest = _reclamationController
        .beginConfigurationSave();
    try {
      final updated = await ref
          .read(storageSettingsGatewayProvider)
          .update(
            catalogDirectory: catalogDirectory,
            previewCacheDirectory: previewDirectory,
            previewBudgetBytes: previewBudgetBytes ?? status.previewBudgetBytes,
          );
      if (mounted) {
        setState(() {
          _isSaving = false;
        });
        await _reclamationController.completeConfigurationSave(
          configurationRequest,
          updated,
        );
      }
    } on Object catch (error) {
      if (mounted) {
        setState(() {
          _isSaving = false;
          _settingsError = error;
        });
      }
    }
  }

  Future<void> _confirmPreviewCleanup([
    RetiredPreviewRootModel? retiredRoot,
  ]) async {
    final cleanup = _cleanupUpdate;
    if (_isSaving || (cleanup != null && cleanup.isActive)) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => StoragePreviewCleanupConfirmation(
        retiredDirectory: retiredRoot?.previewRoot,
        retiredDisplayPath: retiredRoot?.displayPath,
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    _startPreviewCleanup(retiredRoot: retiredRoot);
  }

  void _startPreviewCleanup({RetiredPreviewRootModel? retiredRoot}) {
    final operationId =
        "preview-cleanup-${DateTime.now().microsecondsSinceEpoch}";
    final initial = PreviewCleanupUpdate(
      operationId: operationId,
      phase: PreviewCleanupPhase.started,
      processedFiles: BigInt.zero,
      totalFiles: BigInt.zero,
      removedFiles: BigInt.zero,
      removedBytes: BigInt.zero,
      issueCount: BigInt.zero,
    );
    setState(() {
      _cleanupUpdate = initial;
      _cleanupTargetPreviewRoot = retiredRoot?.previewRoot;
      _cleanupTargetDisplayPath = retiredRoot?.displayPath;
      _previewCleanupErrorMessage = null;
      _isCancellingCleanup = false;
    });
    final gateway = ref.read(storageSettingsGatewayProvider);
    final stream = retiredRoot == null
        ? gateway.clearPreviews(operationId: operationId)
        : gateway.clearRetiredPreviews(
            previewRoot: retiredRoot.previewRoot,
            operationId: operationId,
          );
    _cleanupSubscription = stream.listen(
      _handleCleanupUpdate,
      onError: (Object error) {
        if (!mounted) {
          return;
        }
        setState(() {
          _cleanupUpdate = PreviewCleanupUpdate(
            operationId: operationId,
            phase: PreviewCleanupPhase.failed,
            processedFiles: _cleanupUpdate?.processedFiles ?? BigInt.zero,
            totalFiles: _cleanupUpdate?.totalFiles ?? BigInt.zero,
            removedFiles: _cleanupUpdate?.removedFiles ?? BigInt.zero,
            removedBytes: _cleanupUpdate?.removedBytes ?? BigInt.zero,
            issueCount: _cleanupUpdate?.issueCount ?? BigInt.zero,
            errorMessage: _errorText(error),
          );
          _previewCleanupErrorMessage = _errorText(error);
          _isCancellingCleanup = false;
        });
      },
    );
  }

  void _handleCleanupUpdate(PreviewCleanupUpdate update) {
    if (!mounted) {
      return;
    }
    setState(() {
      _cleanupUpdate = update;
      _previewCleanupErrorMessage = null;
      if (update.isTerminal) {
        _isCancellingCleanup = false;
      }
    });
    if (update.isTerminal) {
      unawaited(_load(errorTarget: _StorageErrorTarget.previewCleanup));
    }
  }

  Future<void> _cancelPreviewCleanup() async {
    final cleanup = _cleanupUpdate;
    if (cleanup == null || !cleanup.isActive || _isCancellingCleanup) {
      return;
    }
    setState(() {
      _isCancellingCleanup = true;
      _previewCleanupErrorMessage = null;
    });
    try {
      final accepted = await ref
          .read(storageSettingsGatewayProvider)
          .cancelPreviewCleanup(operationId: cleanup.operationId);
      if (mounted && !accepted) {
        setState(() {
          _isCancellingCleanup = false;
          _previewCleanupErrorMessage = context.l10n.cleanupAlreadyFinished;
        });
      }
    } on Object catch (error) {
      if (mounted) {
        setState(() {
          _isCancellingCleanup = false;
          _previewCleanupErrorMessage = _errorText(error);
        });
      }
    }
  }

  String _errorText(Object error) {
    if (error case StorageSettingsFailure(:final message)) {
      return message;
    }
    return error.toString();
  }

  void _setError(_StorageErrorTarget target, Object? error) {
    switch (target) {
      case _StorageErrorTarget.settings:
        _settingsError = error;
        break;
      case _StorageErrorTarget.catalogReclamation:
        break;
      case _StorageErrorTarget.previewCleanup:
        _previewCleanupErrorMessage = error == null ? null : _errorText(error);
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = _status;
    if (status == null) {
      return SettingsSection(
        title: context.l10n.storage,
        children: [
          if (_settingsError == null)
            SettingsRow(
              key: Key("storage-settings-loading"),
              icon: Symbols.storage_rounded,
              title: context.l10n.loadingStorageSettings,
              subtitle: Text(context.l10n.checkingStoragePaths),
              trailing: SizedBox.square(
                dimension: 24,
                child: AmeCircularProgress(strokeWidth: 2),
              ),
            )
          else
            SettingsRow(
              key: const Key("storage-settings-load-error"),
              icon: Symbols.error_rounded,
              title: context.l10n.storageSettingsUnavailable,
              subtitle: StorageSettingsError(error: _settingsError!),
              trailing: FilledButton.tonal(
                onPressed: () => unawaited(_load()),
                child: Text(context.l10n.retry),
              ),
            ),
        ],
      );
    }

    final catalogReclamation =
        _reclamation.reclamation ?? status.catalogReclamation;

    return SettingsSection(
      title: context.l10n.storage,
      children: [
        if (status.requiresRestart)
          SettingsRow(
            key: Key("storage-settings-restart-notice"),
            icon: Symbols.restart_alt_rounded,
            title: context.l10n.storageRestartRequired,
            subtitle: Text(context.l10n.existingFilesKept),
          ),
        SettingsRow(
          key: const Key("catalog-location-setting"),
          icon: Symbols.storage_rounded,
          title: context.l10n.catalogStorageLocation,
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                context.l10n.catalogStorageHelp(
                  status.configuredCatalogDisplayPath,
                ),
              ),
              Text(
                context.l10n.storageUsageWithDetail(
                  _formatBytes(status.catalogUsedBytes),
                  status.catalogReclaimableBytes > BigInt.zero
                      ? context.l10n.storageUsageDetail(
                          _formatBytes(status.catalogLiveBytes),
                          _formatBytes(status.catalogReclaimableBytes),
                        )
                      : "",
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
          trailing: widget.hasLibraryRoots
              ? Text(context.l10n.catalogLocationLocked)
              : FilledButton.tonal(
                  onPressed: _isSaving ? null : _chooseCatalogDirectory,
                  child: Text(context.l10n.change),
                ),
        ),
        SettingsRow(
          key: const Key("catalog-reclamation-setting"),
          icon: Symbols.database_rounded,
          title: _catalogReclamationTitle(context.l10n, catalogReclamation),
          subtitle: _catalogReclamationSubtitle(
            context.l10n,
            catalogReclamation,
          ),
          trailing: catalogReclamation.isActive
              ? FilledButton.tonal(
                  onPressed: _reclamation.isActionPending
                      ? null
                      : _reclamationController.cancel,
                  child: Text(
                    _reclamation.isActionPending
                        ? context.l10n.cancellingSource
                        : context.l10n.cancel,
                  ),
                )
              : catalogReclamation.canRetry
              ? FilledButton.tonal(
                  onPressed: _reclamation.isActionPending
                      ? null
                      : _reclamationController.start,
                  child: Text(
                    _reclamation.isActionPending
                        ? context.l10n.starting
                        : catalogReclamation.phase ==
                              CatalogReclamationPhase.idle
                        ? context.l10n.cleanUp
                        : context.l10n.retry,
                  ),
                )
              : Text(context.l10n.cleanupNotNeeded),
        ),
        SettingsRow(
          key: const Key("preview-location-setting"),
          icon: Symbols.photo_library_rounded,
          title: context.l10n.thumbnailLocation,
          subtitle: Text(
            context.l10n.thumbnailStorageHelp(
              status.configuredPreviewDisplayPath,
            ),
          ),
          trailing: FilledButton.tonal(
            onPressed: _isSaving ? null : _choosePreviewDirectory,
            child: Text(context.l10n.change),
          ),
        ),
        for (final retiredRoot in status.retiredPreviewRoots)
          SettingsRow(
            key: ValueKey("retired-preview-root-${retiredRoot.previewRoot}"),
            icon: Symbols.folder_delete_rounded,
            title: context.l10n.retiredThumbnailDirectory,
            subtitle: Text(
              context.l10n.retiredThumbnailsHelp(retiredRoot.displayPath),
            ),
            trailing:
                (_cleanupUpdate?.isActive ?? false) &&
                    _cleanupTargetPreviewRoot == retiredRoot.previewRoot
                ? Text(context.l10n.cleaning)
                : FilledButton.tonal(
                    onPressed: _isSaving || (_cleanupUpdate?.isActive ?? false)
                        ? null
                        : () => _confirmPreviewCleanup(retiredRoot),
                    child: Text(context.l10n.cleanOldFolder),
                  ),
          ),
        SettingsRow(
          key: const Key("preview-budget-setting"),
          icon: Symbols.data_usage_rounded,
          title: context.l10n.thumbnailBudget,
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(context.l10n.thumbnailBudgetHelp),
              const SizedBox(height: 8),
              LinearProgressIndicator(
                value: _usageRatio(
                  status.previewUsedBytes,
                  status.previewBudgetBytes,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                context.l10n.storageUsage(
                  _formatBytes(status.previewUsedBytes),
                ),
              ),
            ],
          ),
          trailing: AmeChoice<BigInt>.menu(
            value: status.previewBudgetBytes,
            selectedLabel: _formatBytes(status.previewBudgetBytes),
            enabled: !_isSaving,
            onSelected: (value) {
              if (value != null && value != status.previewBudgetBytes) {
                _update(previewBudgetBytes: value);
              }
            },
            entries: [
              for (final bytes in _budgetOptions)
                AmeChoiceEntry(value: bytes, label: _formatBytes(bytes)),
            ],
          ),
        ),
        SettingsRow(
          key: const Key("preview-cleanup-setting"),
          icon: Symbols.cleaning_services_rounded,
          title: _cleanupTitle(
            context.l10n,
            _cleanupUpdate,
            isRetiredRoot: _cleanupTargetPreviewRoot != null,
          ),
          subtitle: _cleanupSubtitle(
            context.l10n,
            _cleanupUpdate,
            targetDisplayPath: _cleanupTargetDisplayPath,
          ),
          trailing: _cleanupUpdate?.isActive ?? false
              ? FilledButton.tonal(
                  onPressed: _isCancellingCleanup
                      ? null
                      : _cancelPreviewCleanup,
                  child: Text(
                    _isCancellingCleanup
                        ? context.l10n.cancellingSource
                        : context.l10n.cancel,
                  ),
                )
              : FilledButton.tonal(
                  onPressed: _isSaving ? null : () => _confirmPreviewCleanup(),
                  child: Text(context.l10n.cleanUp),
                ),
        ),
        if (_settingsError case final error?)
          SettingsRow(
            key: const Key("storage-settings-error"),
            icon: Symbols.error_rounded,
            title: context.l10n.storageSaveFailed,
            subtitle: StorageSettingsError(error: error),
          ),
        if (_reclamation.errorMessage != null)
          SettingsRow(
            key: const Key("catalog-reclamation-error"),
            icon: Symbols.error_rounded,
            title: context.l10n.catalogMaintenanceStatusError,
            subtitle: Text(
              _reclamation.errorMessage!.resolve(context.l10n),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
            trailing: FilledButton.tonal(
              onPressed: _reclamationController.retry,
              child: Text(context.l10n.retry),
            ),
          ),
        if (_previewCleanupErrorMessage != null)
          SettingsRow(
            key: const Key("preview-cleanup-error"),
            icon: Symbols.error_rounded,
            title: context.l10n.thumbnailCleanupStatusError,
            subtitle: Text(
              _previewCleanupErrorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (_isSaving)
          SettingsRow(
            key: Key("storage-settings-saving"),
            icon: Symbols.sync_rounded,
            title: context.l10n.saving,
            subtitle: Text(context.l10n.storageLocationPending),
            trailing: SizedBox.square(
              dimension: 24,
              child: AmeCircularProgress(strokeWidth: 2),
            ),
          ),
      ],
    );
  }
}

String _catalogReclamationTitle(
  AppLocalizations strings,
  CatalogReclamationModel reclamation,
) {
  return switch (reclamation.phase) {
    CatalogReclamationPhase.idle => strings.optimizeCatalog,
    CatalogReclamationPhase.queued => strings.catalogWaiting,
    CatalogReclamationPhase.inspecting => strings.catalogInspecting,
    CatalogReclamationPhase.waitingForIdle => strings.catalogWaitingIdle,
    CatalogReclamationPhase.checkingCapacity => strings.catalogCheckingSpace,
    CatalogReclamationPhase.converting => strings.catalogOptimizing,
    CatalogReclamationPhase.reclaiming => strings.catalogReclaiming,
    CatalogReclamationPhase.completed => strings.catalogOptimized,
    CatalogReclamationPhase.cancelled => strings.catalogOptimizationCancelled,
    CatalogReclamationPhase.failed => strings.catalogOptimizationFailed,
  };
}

Widget _catalogReclamationSubtitle(
  AppLocalizations strings,
  CatalogReclamationModel reclamation,
) {
  final total = reclamation.reclaimedBytes + reclamation.reclaimableBytes;
  final progress =
      reclamation.phase == CatalogReclamationPhase.reclaiming &&
          total > BigInt.zero
      ? (reclamation.reclaimedBytes.toDouble() / total.toDouble())
            .clamp(0, 1)
            .toDouble()
      : null;
  final status = switch (reclamation.phase) {
    CatalogReclamationPhase.idle =>
      reclamation.reclaimableBytes > BigInt.zero
          ? strings.reclaimableSpace(_formatBytes(reclamation.reclaimableBytes))
          : strings.catalogAutoReclaim,
    CatalogReclamationPhase.queued => strings.catalogRemovalApplied,
    CatalogReclamationPhase.inspecting => strings.catalogEstimating,
    CatalogReclamationPhase.waitingForIdle => strings.catalogBusyWait,
    CatalogReclamationPhase.checkingCapacity =>
      strings.catalogCheckingTempSpace,
    CatalogReclamationPhase.converting => strings.catalogConverting(
      _formatBytes(reclamation.reclaimableBytes),
    ),
    CatalogReclamationPhase.reclaiming => strings.catalogReleasedRemaining(
      _formatBytes(reclamation.reclaimedBytes),
      _formatBytes(reclamation.reclaimableBytes),
    ),
    CatalogReclamationPhase.completed => strings.catalogReleased(
      _formatBytes(reclamation.reclaimedBytes),
    ),
    CatalogReclamationPhase.cancelled => strings.catalogStopped,
    CatalogReclamationPhase.failed =>
      reclamation.errorMessage ?? strings.catalogRetryLater,
  };
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(status),
      if (reclamation.phase == CatalogReclamationPhase.failed &&
          reclamation.requiredTemporaryBytes != null &&
          reclamation.availableTemporaryBytes != null) ...[
        const SizedBox(height: 4),
        Text(
          strings.spaceRequired(
            _formatBytes(reclamation.requiredTemporaryBytes!),
            _formatBytes(reclamation.availableTemporaryBytes!),
          ),
        ),
      ],
      const SizedBox(height: 8),
      AmeLinearProgress(active: reclamation.isActive, value: progress),
    ],
  );
}

String _cleanupTitle(
  AppLocalizations strings,
  PreviewCleanupUpdate? update, {
  required bool isRetiredRoot,
}) {
  final subject = isRetiredRoot
      ? strings.retiredThumbnailDirectory
      : strings.thumbnails;
  return switch (update?.phase) {
    PreviewCleanupPhase.started ||
    PreviewCleanupPhase.running => strings.cleaningSubject(subject),
    PreviewCleanupPhase.completed => strings.cleanupSubjectComplete(subject),
    PreviewCleanupPhase.cancelled => strings.cleanupSubjectCancelled(subject),
    PreviewCleanupPhase.failed => strings.cleanupSubjectFailed(subject),
    null => strings.clearThumbnails,
  };
}

Widget _cleanupSubtitle(
  AppLocalizations strings,
  PreviewCleanupUpdate? update, {
  required String? targetDisplayPath,
}) {
  if (update == null) {
    return Text(strings.thumbnailCleanupHelp);
  }
  final progress = update.totalFiles == BigInt.zero
      ? null
      : (update.processedFiles.toDouble() / update.totalFiles.toDouble())
            .clamp(0, 1)
            .toDouble();
  final status = switch (update.phase) {
    PreviewCleanupPhase.started => strings.countingThumbnails,
    PreviewCleanupPhase.running => strings.cleanupFileProgress(
      update.processedFiles.toString(),
      update.totalFiles.toString(),
      _formatBytes(update.removedBytes),
    ),
    PreviewCleanupPhase.completed => strings.cleanupFilesRemoved(
      update.removedFiles.toString(),
      _formatBytes(update.removedBytes),
    ),
    PreviewCleanupPhase.cancelled => strings.cleanupBeforeStop(
      update.removedFiles.toString(),
    ),
    PreviewCleanupPhase.failed =>
      update.errorMessage ?? strings.thumbnailCleanupFailed,
  };
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      if (targetDisplayPath != null) ...[
        Text(targetDisplayPath),
        const SizedBox(height: 4),
      ],
      Text(status),
      const SizedBox(height: 8),
      AmeLinearProgress(active: update.isActive, value: progress),
      if (update.issueCount > BigInt.zero) ...[
        const SizedBox(height: 6),
        Text(strings.cleanupFailedFiles(update.issueCount.toString())),
      ],
      if (update.issueMessage != null) ...[
        const SizedBox(height: 4),
        Text(update.issueMessage!),
      ],
    ],
  );
}

final _budgetOptions = <BigInt>[
  for (final gibibytes in [1, 2, 4, 8, 16, 32])
    BigInt.from(gibibytes) * BigInt.from(1024 * 1024 * 1024),
];

double _usageRatio(BigInt used, BigInt budget) {
  if (budget <= BigInt.zero) {
    return 0;
  }
  return (used.toDouble() / budget.toDouble()).clamp(0, 1);
}

String _formatBytes(BigInt bytes) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  var value = bytes.toDouble();
  var unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  final digits = value == value.roundToDouble() || value >= 10 || unit == 0
      ? 0
      : 1;
  return "${value.toStringAsFixed(digits)} ${units[unit]}";
}
