import "dart:async";
import "dart:io";

import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../storage/application/storage_settings.dart";
import "../../../storage/domain/storage_models.dart";
import "../../../storage/presentation/catalog_reclamation_controller.dart";
import "settings_section.dart";

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
  String? _settingsErrorMessage;
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
          setState(() => _setError(effectiveTarget, _errorText(error)));
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
        .pick(initialDirectory: File(status.configuredCatalogPath).parent.path);
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
        .pick(
          initialDirectory: Directory(status.configuredPreviewRoot).parent.path,
        );
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
      _settingsErrorMessage = null;
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
          _settingsErrorMessage = _errorText(error);
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
      builder: (context) {
        return AlertDialog(
          title: Text(retiredRoot == null ? "清理缩略图？" : "清理旧缩略图目录？"),
          content: Text(
            retiredRoot == null
                ? "这会删除可重新生成的缩略图缓存，不会删除或修改原图片。"
                      "清理后，打开图库时可能需要一些时间重新生成缩略图。"
                : "只会删除旧目录中由 Ame 管理的缩略图，不会删除原图片或目录中的其他文件。\n"
                      "${retiredRoot.displayPath}",
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text("取消"),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text("开始清理"),
            ),
          ],
        );
      },
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
          _previewCleanupErrorMessage = "清理任务已经结束，无法再取消";
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

  void _setError(_StorageErrorTarget target, String? message) {
    switch (target) {
      case _StorageErrorTarget.settings:
        _settingsErrorMessage = message;
        break;
      case _StorageErrorTarget.catalogReclamation:
        break;
      case _StorageErrorTarget.previewCleanup:
        _previewCleanupErrorMessage = message;
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = _status;
    if (status == null) {
      return SettingsSection(
        title: "存储",
        children: [
          if (_settingsErrorMessage == null)
            const SettingsRow(
              key: Key("storage-settings-loading"),
              icon: Symbols.storage_rounded,
              title: "正在读取存储设置",
              subtitle: Text("正在检查图库数据与缩略图的保存位置"),
              trailing: SizedBox.square(
                dimension: 24,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            SettingsRow(
              key: const Key("storage-settings-load-error"),
              icon: Symbols.error_rounded,
              title: "无法读取存储设置",
              subtitle: Text(_settingsErrorMessage!),
              trailing: OutlinedButton(
                onPressed: () => unawaited(_load()),
                child: const Text("重试"),
              ),
            ),
        ],
      );
    }

    final catalogReclamation =
        _reclamation.reclamation ?? status.catalogReclamation;

    return SettingsSection(
      title: "存储",
      children: [
        if (status.requiresRestart)
          const SettingsRow(
            key: Key("storage-settings-restart-notice"),
            icon: Symbols.restart_alt_rounded,
            title: "重启 Ame 后应用新的存储设置",
            subtitle: Text("现有文件不会被移动或删除"),
          ),
        SettingsRow(
          key: const Key("catalog-location-setting"),
          icon: Symbols.storage_rounded,
          title: "图库数据位置",
          subtitle: Text(
            "保存图库索引和扫描结果\n"
            "${status.configuredCatalogDisplayPath}\n"
            "当前占用 ${_formatBytes(status.catalogUsedBytes)}"
            "${status.catalogReclaimableBytes > BigInt.zero ? "\n有效数据约 ${_formatBytes(status.catalogLiveBytes)}，可回收 ${_formatBytes(status.catalogReclaimableBytes)}" : ""}",
          ),
          trailing: widget.hasLibraryRoots
              ? const TextButton(onPressed: null, child: Text("已有图库时不可更改"))
              : OutlinedButton(
                  onPressed: _isSaving ? null : _chooseCatalogDirectory,
                  child: const Text("更改"),
                ),
        ),
        SettingsRow(
          key: const Key("catalog-reclamation-setting"),
          icon: Symbols.database_rounded,
          title: _catalogReclamationTitle(catalogReclamation),
          subtitle: _catalogReclamationSubtitle(catalogReclamation),
          trailing: catalogReclamation.isActive
              ? OutlinedButton(
                  onPressed: _reclamation.isActionPending
                      ? null
                      : _reclamationController.cancel,
                  child: Text(_reclamation.isActionPending ? "正在取消" : "取消"),
                )
              : catalogReclamation.canRetry
              ? OutlinedButton(
                  onPressed: _reclamation.isActionPending
                      ? null
                      : _reclamationController.start,
                  child: Text(
                    _reclamation.isActionPending
                        ? "正在启动"
                        : catalogReclamation.phase ==
                              CatalogReclamationPhase.idle
                        ? "清理"
                        : "重试",
                  ),
                )
              : const TextButton(onPressed: null, child: Text("无需清理")),
        ),
        SettingsRow(
          key: const Key("preview-location-setting"),
          icon: Symbols.photo_library_rounded,
          title: "缩略图位置",
          subtitle: Text(
            "保存可随时重新生成的图片预览\n"
            "${status.configuredPreviewDisplayPath}",
          ),
          trailing: OutlinedButton(
            onPressed: _isSaving ? null : _choosePreviewDirectory,
            child: const Text("更改"),
          ),
        ),
        for (final retiredRoot in status.retiredPreviewRoots)
          SettingsRow(
            key: ValueKey("retired-preview-root-${retiredRoot.previewRoot}"),
            icon: Symbols.folder_delete_rounded,
            title: "旧缩略图目录",
            subtitle: Text(
              "新目录已启用；旧目录只会在确认后清理 Ame 管理的缩略图\n"
              "${retiredRoot.displayPath}",
            ),
            trailing:
                (_cleanupUpdate?.isActive ?? false) &&
                    _cleanupTargetPreviewRoot == retiredRoot.previewRoot
                ? const TextButton(onPressed: null, child: Text("正在清理"))
                : OutlinedButton(
                    onPressed: _isSaving || (_cleanupUpdate?.isActive ?? false)
                        ? null
                        : () => _confirmPreviewCleanup(retiredRoot),
                    child: const Text("清理旧目录"),
                  ),
          ),
        SettingsRow(
          key: const Key("preview-budget-setting"),
          icon: Symbols.data_usage_rounded,
          title: "缩略图最大占用空间",
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text("达到上限时自动回收当前不需要的缩略图，不会修改原图片"),
              const SizedBox(height: 8),
              LinearProgressIndicator(
                value: _usageRatio(
                  status.previewUsedBytes,
                  status.previewBudgetBytes,
                ),
              ),
              const SizedBox(height: 6),
              Text("当前占用 ${_formatBytes(status.previewUsedBytes)}"),
            ],
          ),
          trailing: SettingsChoice<BigInt>(
            value: status.previewBudgetBytes,
            width: 144,
            selectedLabel: _formatBytes(status.previewBudgetBytes),
            enabled: !_isSaving,
            onSelected: (value) {
              if (value != null && value != status.previewBudgetBytes) {
                _update(previewBudgetBytes: value);
              }
            },
            entries: [
              for (final bytes in _budgetOptions)
                SettingsChoiceEntry(value: bytes, label: _formatBytes(bytes)),
            ],
          ),
        ),
        SettingsRow(
          key: const Key("preview-cleanup-setting"),
          icon: Symbols.cleaning_services_rounded,
          title: _cleanupTitle(
            _cleanupUpdate,
            isRetiredRoot: _cleanupTargetPreviewRoot != null,
          ),
          subtitle: _cleanupSubtitle(
            _cleanupUpdate,
            targetDisplayPath: _cleanupTargetDisplayPath,
          ),
          trailing: _cleanupUpdate?.isActive ?? false
              ? OutlinedButton(
                  onPressed: _isCancellingCleanup
                      ? null
                      : _cancelPreviewCleanup,
                  child: Text(_isCancellingCleanup ? "正在取消" : "取消"),
                )
              : OutlinedButton(
                  onPressed: _isSaving ? null : () => _confirmPreviewCleanup(),
                  child: const Text("清理"),
                ),
        ),
        if (_settingsErrorMessage != null)
          SettingsRow(
            key: const Key("storage-settings-error"),
            icon: Symbols.error_rounded,
            title: "未能保存存储设置",
            subtitle: Text(
              _settingsErrorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (_reclamation.errorMessage != null)
          SettingsRow(
            key: const Key("catalog-reclamation-error"),
            icon: Symbols.error_rounded,
            title: "图库数据整理状态异常",
            subtitle: Text(
              _reclamation.errorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
            trailing: OutlinedButton(
              onPressed: _reclamationController.retry,
              child: const Text("重试"),
            ),
          ),
        if (_previewCleanupErrorMessage != null)
          SettingsRow(
            key: const Key("preview-cleanup-error"),
            icon: Symbols.error_rounded,
            title: "缩略图清理状态异常",
            subtitle: Text(
              _previewCleanupErrorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (_isSaving)
          const SettingsRow(
            key: Key("storage-settings-saving"),
            icon: Symbols.sync_rounded,
            title: "正在保存",
            subtitle: Text("完成前不会改变当前正在使用的存储位置"),
            trailing: SizedBox.square(
              dimension: 24,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
      ],
    );
  }
}

String _catalogReclamationTitle(CatalogReclamationModel reclamation) {
  return switch (reclamation.phase) {
    CatalogReclamationPhase.idle => "整理图库数据",
    CatalogReclamationPhase.queued => "图库数据等待整理",
    CatalogReclamationPhase.inspecting => "正在检查图库数据",
    CatalogReclamationPhase.waitingForIdle => "等待空闲后整理图库数据",
    CatalogReclamationPhase.checkingCapacity => "正在检查整理空间",
    CatalogReclamationPhase.converting => "正在优化图库数据库",
    CatalogReclamationPhase.reclaiming => "正在释放图库数据空间",
    CatalogReclamationPhase.completed => "图库数据整理完成",
    CatalogReclamationPhase.cancelled => "图库数据整理已取消",
    CatalogReclamationPhase.failed => "图库数据整理失败",
  };
}

Widget _catalogReclamationSubtitle(CatalogReclamationModel reclamation) {
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
          ? "约 ${_formatBytes(reclamation.reclaimableBytes)} 空间可以安全回收"
          : "删除图库后会在后台回收数据库空闲页，不会删除原图片",
    CatalogReclamationPhase.queued => "移除结果已经生效，后台整理即将开始",
    CatalogReclamationPhase.inspecting => "正在计算有效数据与可回收空间",
    CatalogReclamationPhase.waitingForIdle => "有更重要的图库操作正在进行，稍后自动继续",
    CatalogReclamationPhase.checkingCapacity => "正在确认数据库重建所需的临时磁盘空间",
    CatalogReclamationPhase.converting =>
      "正在执行一次性数据库转换，可回收约 ${_formatBytes(reclamation.reclaimableBytes)}；此阶段无法可靠估算百分比",
    CatalogReclamationPhase.reclaiming =>
      "已释放 ${_formatBytes(reclamation.reclaimedBytes)}，剩余约 ${_formatBytes(reclamation.reclaimableBytes)}",
    CatalogReclamationPhase.completed =>
      "已释放 ${_formatBytes(reclamation.reclaimedBytes)}；有效图库数据保持不变",
    CatalogReclamationPhase.cancelled => "已停止整理；图库移除结果与有效数据不受影响，可稍后重试",
    CatalogReclamationPhase.failed =>
      reclamation.errorMessage ?? "未能完成图库数据整理，可稍后重试",
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
          "需要 ${_formatBytes(reclamation.requiredTemporaryBytes!)}，"
          "当前可用 ${_formatBytes(reclamation.availableTemporaryBytes!)}",
        ),
      ],
      if (reclamation.isActive) ...[
        const SizedBox(height: 8),
        LinearProgressIndicator(value: progress),
      ],
    ],
  );
}

String _cleanupTitle(
  PreviewCleanupUpdate? update, {
  required bool isRetiredRoot,
}) {
  final subject = isRetiredRoot ? "旧缩略图目录" : "缩略图";
  return switch (update?.phase) {
    PreviewCleanupPhase.started ||
    PreviewCleanupPhase.running => "正在清理$subject",
    PreviewCleanupPhase.completed => "$subject清理完成",
    PreviewCleanupPhase.cancelled => "$subject清理已取消",
    PreviewCleanupPhase.failed => "$subject清理失败",
    null => "清理缩略图",
  };
}

Widget _cleanupSubtitle(
  PreviewCleanupUpdate? update, {
  required String? targetDisplayPath,
}) {
  if (update == null) {
    return const Text("缩略图会在需要时重新生成，不会删除原图片");
  }
  final progress = update.totalFiles == BigInt.zero
      ? null
      : (update.processedFiles.toDouble() / update.totalFiles.toDouble())
            .clamp(0, 1)
            .toDouble();
  final status = switch (update.phase) {
    PreviewCleanupPhase.started => "正在统计可清理的缩略图",
    PreviewCleanupPhase.running =>
      "已处理 ${update.processedFiles} / ${update.totalFiles} 个文件，"
          "释放 ${_formatBytes(update.removedBytes)}",
    PreviewCleanupPhase.completed =>
      "已移除 ${update.removedFiles} 个文件，释放 ${_formatBytes(update.removedBytes)}",
    PreviewCleanupPhase.cancelled => "停止前已移除 ${update.removedFiles} 个文件，已保留原图片",
    PreviewCleanupPhase.failed => update.errorMessage ?? "未能完成缩略图清理",
  };
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      if (targetDisplayPath != null) ...[
        Text(targetDisplayPath),
        const SizedBox(height: 4),
      ],
      Text(status),
      if (update.isActive) ...[
        const SizedBox(height: 8),
        LinearProgressIndicator(value: progress),
      ],
      if (update.issueCount > BigInt.zero) ...[
        const SizedBox(height: 6),
        Text("${update.issueCount} 个文件未能清理"),
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
