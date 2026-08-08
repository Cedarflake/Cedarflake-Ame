import "dart:io";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

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
  String? _errorMessage;
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _load();
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
              icon: Icons.storage_outlined,
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
              icon: Icons.error_outline,
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
            icon: Icons.restart_alt,
            title: "重启 Ame 后应用新的存储设置",
            subtitle: Text("现有文件不会被移动或删除"),
          ),
        SettingsRow(
          key: const Key("catalog-location-setting"),
          icon: Icons.storage_outlined,
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
          icon: Icons.photo_library_outlined,
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
        SettingsRow(
          key: const Key("preview-budget-setting"),
          icon: Icons.data_usage_outlined,
          title: "缩略图最大占用空间",
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text("达到上限后停止生成新的缩略图，不会修改原图片"),
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
        if (_errorMessage != null)
          SettingsRow(
            key: const Key("storage-settings-error"),
            icon: Icons.error_outline,
            title: "未能保存存储设置",
            subtitle: Text(
              _errorMessage!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (_isSaving)
          const SettingsRow(
            key: Key("storage-settings-saving"),
            icon: Icons.sync,
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
