import "dart:async";
import "dart:io";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../storage/application/storage_settings.dart";
import "../../../storage/domain/storage_models.dart";
import "settings_section.dart";

class StorageSettingsSection extends ConsumerStatefulWidget {
  const StorageSettingsSection({required this.hasLibraryRoots, super.key});

  final bool hasLibraryRoots;

  @override
  ConsumerState<StorageSettingsSection> createState() =>
      _StorageSettingsSectionState();
}

class _StorageSettingsSectionState
    extends ConsumerState<StorageSettingsSection> {
  StorageStatusModel? _status;
  PreviewCleanupUpdate? _cleanupUpdate;
  StreamSubscription<PreviewCleanupUpdate>? _cleanupSubscription;
  String? _cleanupTargetPreviewRoot;
  String? _cleanupTargetDisplayPath;
  String? _errorMessage;
  bool _isSaving = false;
  bool _isCancellingCleanup = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    final cleanup = _cleanupUpdate;
    unawaited(_cleanupSubscription?.cancel());
    if (cleanup != null && cleanup.isActive) {
      unawaited(
        ref
            .read(storageSettingsGatewayProvider)
            .cancelPreviewCleanup(operationId: cleanup.operationId),
      );
    }
    super.dispose();
  }

  Future<void> _load() async {
    setState(() => _errorMessage = null);
    try {
      final status = await ref.read(storageSettingsGatewayProvider).load();
      if (mounted) {
        setState(() => _status = status);
      }
    } on Object catch (error) {
      if (mounted) {
        setState(() => _errorMessage = _errorText(error));
      }
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
      _errorMessage = null;
    });
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
          _status = updated;
          _isSaving = false;
        });
      }
    } on Object catch (error) {
      if (mounted) {
        setState(() {
          _isSaving = false;
          _errorMessage = _errorText(error);
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
      _errorMessage = null;
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
      if (update.isTerminal) {
        _isCancellingCleanup = false;
      }
    });
    if (update.isTerminal) {
      unawaited(_load());
    }
  }

  Future<void> _cancelPreviewCleanup() async {
    final cleanup = _cleanupUpdate;
    if (cleanup == null || !cleanup.isActive || _isCancellingCleanup) {
      return;
    }
    setState(() => _isCancellingCleanup = true);
    try {
      final accepted = await ref
          .read(storageSettingsGatewayProvider)
          .cancelPreviewCleanup(operationId: cleanup.operationId);
      if (mounted && !accepted) {
        setState(() {
          _isCancellingCleanup = false;
          _errorMessage = "清理任务已经结束，无法再取消";
        });
      }
    } on Object catch (error) {
      if (mounted) {
        setState(() {
          _isCancellingCleanup = false;
          _errorMessage = _errorText(error);
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

  @override
  Widget build(BuildContext context) {
    final status = _status;
    if (status == null) {
      return SettingsSection(
        title: "存储",
        children: [
          if (_errorMessage == null)
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
              subtitle: Text(_errorMessage!),
              trailing: OutlinedButton(
                onPressed: _load,
                child: const Text("重试"),
              ),
            ),
        ],
      );
    }

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
            "当前占用 ${_formatBytes(status.catalogUsedBytes)}",
          ),
          trailing: widget.hasLibraryRoots
              ? const TextButton(onPressed: null, child: Text("已有图库时不可更改"))
              : OutlinedButton(
                  onPressed: _isSaving ? null : _chooseCatalogDirectory,
                  child: const Text("更改"),
                ),
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
          trailing: DropdownMenu<BigInt>(
            key: ValueKey(status.previewBudgetBytes),
            width: 144,
            initialSelection: status.previewBudgetBytes,
            enabled: !_isSaving,
            enableSearch: false,
            requestFocusOnTap: false,
            selectOnly: true,
            trailingIcon: const Icon(Symbols.arrow_drop_down_rounded),
            selectedTrailingIcon: const Icon(Symbols.arrow_drop_up_rounded),
            onSelected: (value) {
              if (value != null && value != status.previewBudgetBytes) {
                _update(previewBudgetBytes: value);
              }
            },
            dropdownMenuEntries: [
              for (final bytes in _budgetOptions)
                DropdownMenuEntry(value: bytes, label: _formatBytes(bytes)),
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
        if (_errorMessage != null)
          SettingsRow(
            key: const Key("storage-settings-error"),
            icon: Symbols.error_rounded,
            title: "未能保存存储设置",
            subtitle: Text(
              _errorMessage!,
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
