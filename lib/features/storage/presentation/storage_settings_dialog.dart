import "dart:io";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../application/storage_settings.dart";
import "../domain/storage_models.dart";

class StorageSettingsButton extends ConsumerWidget {
  const StorageSettingsButton({required this.hasLibraryRoots, super.key});

  final bool hasLibraryRoots;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return IconButton(
      key: const Key("storage-settings-button"),
      tooltip: "Settings",
      onPressed: () {
        showDialog<void>(
          context: context,
          builder: (context) =>
              StorageSettingsDialog(hasLibraryRoots: hasLibraryRoots),
        );
      },
      icon: const Icon(Icons.settings_outlined),
    );
  }
}

class StorageSettingsDialog extends ConsumerStatefulWidget {
  const StorageSettingsDialog({required this.hasLibraryRoots, super.key});

  final bool hasLibraryRoots;

  @override
  ConsumerState<StorageSettingsDialog> createState() =>
      _StorageSettingsDialogState();
}

class _StorageSettingsDialogState extends ConsumerState<StorageSettingsDialog> {
  StorageStatusModel? _status;
  String? _catalogDirectory;
  String? _previewDirectory;
  BigInt? _previewBudgetBytes;
  String? _errorMessage;
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final status = await ref.read(storageSettingsGatewayProvider).load();
      if (!mounted) {
        return;
      }
      setState(() {
        _status = status;
        _previewBudgetBytes = status.previewBudgetBytes;
        _errorMessage = null;
      });
    } on Object catch (error) {
      if (mounted) {
        setState(() => _errorMessage = _errorText(error));
      }
    }
  }

  Future<void> _chooseCatalogDirectory() async {
    final status = _status;
    if (status == null || widget.hasLibraryRoots) {
      return;
    }
    final directory = await ref
        .read(storageDirectoryPickerProvider)
        .pick(initialDirectory: File(status.configuredCatalogPath).parent.path);
    if (directory != null && mounted) {
      setState(() => _catalogDirectory = directory);
    }
  }

  Future<void> _choosePreviewDirectory() async {
    final status = _status;
    if (status == null) {
      return;
    }
    final directory = await ref
        .read(storageDirectoryPickerProvider)
        .pick(
          initialDirectory: Directory(status.configuredPreviewRoot).parent.path,
        );
    if (directory != null && mounted) {
      setState(() => _previewDirectory = directory);
    }
  }

  Future<void> _save() async {
    final budget = _previewBudgetBytes;
    if (_status == null || budget == null || _isSaving) {
      return;
    }
    setState(() {
      _isSaving = true;
      _errorMessage = null;
    });
    try {
      final status = await ref
          .read(storageSettingsGatewayProvider)
          .update(
            catalogDirectory: _catalogDirectory,
            previewCacheDirectory: _previewDirectory,
            previewBudgetBytes: budget,
          );
      if (!mounted) {
        return;
      }
      setState(() {
        _status = status;
        _catalogDirectory = null;
        _previewDirectory = null;
        _isSaving = false;
      });
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
    return AlertDialog(
      key: const Key("storage-settings-dialog"),
      title: const Text("Storage"),
      content: SizedBox(
        width: 680,
        child: status == null
            ? _LoadingOrError(errorMessage: _errorMessage, onRetry: _load)
            : SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (status.requiresRestart) ...[
                      const _RestartBanner(),
                      const SizedBox(height: 20),
                    ],
                    _StorageLocation(
                      title: "Catalog database",
                      activePath: status.activeCatalogPath,
                      configuredPath: _catalogDirectory == null
                          ? status.configuredCatalogPath
                          : Directory(
                              _catalogDirectory!,
                            ).uri.resolve("ame.sqlite3").toFilePath(),
                      usedBytes: status.catalogUsedBytes,
                      actionLabel: widget.hasLibraryRoots
                          ? "Migration required"
                          : "Choose folder",
                      onChoose: widget.hasLibraryRoots
                          ? null
                          : _chooseCatalogDirectory,
                    ),
                    if (widget.hasLibraryRoots)
                      const Padding(
                        padding: EdgeInsets.only(top: 8),
                        child: Text(
                          "Catalog relocation is locked after sources are imported until a verified migration workflow is available.",
                        ),
                      ),
                    const SizedBox(height: 24),
                    _StorageLocation(
                      title: "Preview cache",
                      activePath: status.activePreviewRoot,
                      configuredPath: _previewDirectory == null
                          ? status.configuredPreviewRoot
                          : Directory(
                              _previewDirectory!,
                            ).uri.resolve("ame-jpeg-thumbnail-v1").toFilePath(),
                      usedBytes: status.previewUsedBytes,
                      actionLabel: "Choose folder",
                      onChoose: _choosePreviewDirectory,
                    ),
                    const SizedBox(height: 16),
                    DropdownButtonFormField<BigInt>(
                      key: const Key("preview-budget-field"),
                      initialValue: _previewBudgetBytes,
                      decoration: const InputDecoration(
                        border: OutlineInputBorder(),
                        labelText: "Preview cache budget",
                      ),
                      items: _budgetOptions
                          .map(
                            (bytes) => DropdownMenuItem(
                              value: bytes,
                              child: Text(_formatBytes(bytes)),
                            ),
                          )
                          .toList(),
                      onChanged: _isSaving
                          ? null
                          : (value) {
                              if (value != null) {
                                setState(() => _previewBudgetBytes = value);
                              }
                            },
                    ),
                    const SizedBox(height: 8),
                    LinearProgressIndicator(
                      value: _usageRatio(
                        status.previewUsedBytes,
                        _previewBudgetBytes ?? status.previewBudgetBytes,
                      ),
                    ),
                    const SizedBox(height: 6),
                    Text(
                      "${_formatBytes(status.previewUsedBytes)} currently used. New previews stop safely when the budget is exhausted.",
                    ),
                    if (_errorMessage != null) ...[
                      const SizedBox(height: 16),
                      Text(
                        _errorMessage!,
                        key: const Key("storage-settings-error"),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ],
                    const SizedBox(height: 16),
                    Text(
                      "Settings file: ${status.settingsPath}",
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text("Close"),
        ),
        FilledButton(
          key: const Key("storage-settings-save-button"),
          onPressed: status == null || _isSaving ? null : _save,
          child: Text(_isSaving ? "Saving" : "Save"),
        ),
      ],
    );
  }
}

class _StorageLocation extends StatelessWidget {
  const _StorageLocation({
    required this.title,
    required this.activePath,
    required this.configuredPath,
    required this.usedBytes,
    required this.actionLabel,
    required this.onChoose,
  });

  final String title;
  final String activePath;
  final String configuredPath;
  final BigInt usedBytes;
  final String actionLabel;
  final VoidCallback? onChoose;

  @override
  Widget build(BuildContext context) {
    final isPending = activePath != configuredPath;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                title,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            OutlinedButton.icon(
              onPressed: onChoose,
              icon: const Icon(Icons.folder_open_outlined),
              label: Text(actionLabel),
            ),
          ],
        ),
        const SizedBox(height: 8),
        SelectableText(configuredPath),
        const SizedBox(height: 4),
        Text(
          isPending
              ? "Active until restart: $activePath"
              : "${_formatBytes(usedBytes)} used",
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _RestartBanner extends StatelessWidget {
  const _RestartBanner();

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(12),
      child: const Padding(
        padding: EdgeInsets.all(14),
        child: Row(
          children: [
            Icon(Icons.restart_alt),
            SizedBox(width: 12),
            Expanded(
              child: Text(
                "Saved. Restart Ame to activate the configured storage. Existing files are not moved or deleted.",
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _LoadingOrError extends StatelessWidget {
  const _LoadingOrError({required this.errorMessage, required this.onRetry});

  final String? errorMessage;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    if (errorMessage == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(errorMessage!, key: const Key("storage-settings-load-error")),
        const SizedBox(height: 12),
        OutlinedButton(onPressed: onRetry, child: const Text("Retry")),
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
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
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
