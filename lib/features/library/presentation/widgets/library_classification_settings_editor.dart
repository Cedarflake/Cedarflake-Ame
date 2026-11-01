import "dart:async";
import "dart:math";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_model_installation.dart";
import "library_classification_category_editor.dart";

Future<void> showLibraryClassificationSettings(BuildContext context) async {
  final saved = await showDialog<bool>(
    context: context,
    builder: (context) => const _SettingsEditor(),
  );
  if (context.mounted && saved == true) {
    showAmeTransientFeedback(
      context,
      message: "智能相册设置已保存",
      severity: AmeNotificationSeverity.success,
    );
  }
}

class _SettingsEditor extends ConsumerStatefulWidget {
  const _SettingsEditor();
  @override
  ConsumerState<_SettingsEditor> createState() => _SettingsEditorState();
}

class _SettingsEditorState extends ConsumerState<_SettingsEditor> {
  LibraryClassificationSettings? _observed;
  List<LibraryClassificationCategory> _categories = [];
  String? _error;
  String? _operationId;
  bool? _attemptEnable;
  bool _busy = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final service = ref.read(libraryClassificationConfigurationProvider);
      if (service == null) throw StateError("Classification unavailable");
      final settings = await service.load();
      if (mounted) {
        setState(() {
          _observed = settings;
          _categories = List.of(settings.categories);
          _operationId = null;
        });
      }
    } on Object {
      if (mounted) setState(() => _error = "无法读取智能相册设置，请重试。");
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _edit([int? index]) async {
    final category = await showLibraryClassificationCategoryEditor(
      context,
      index == null ? null : _categories[index],
    );
    if (!mounted || category == null) return;
    setState(() {
      if (index == null) {
        _categories.add(category);
      } else {
        _categories[index] = category;
      }
      _operationId = null;
    });
  }

  Future<void> _save({required bool enable}) async {
    final observed = _observed;
    final service = ref.read(libraryClassificationConfigurationProvider);
    if (_busy || observed == null || service == null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    final random = Random.secure();
    if (_attemptEnable != enable) _operationId = null;
    _attemptEnable = enable;
    _operationId ??= List.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
    ).join();
    try {
      await service.save(
        operationId: _operationId!,
        observed: observed,
        categories: List.unmodifiable(_categories),
        action: enable
            ? LibraryClassificationConfigurationAction.enable
            : LibraryClassificationConfigurationAction.save,
      );
      if (!mounted) return;
      ref.invalidate(libraryClassificationSettingsProvider);
      Navigator.of(context).pop(true);
    } on Object {
      if (mounted) setState(() => _error = "设置未能保存，请重新加载设置后重试。");
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _install() {
    final observed = _observed;
    final configuration = ref.read(libraryClassificationConfigurationProvider);
    final installation = ref.read(libraryModelInstallationProvider);
    if (_busy ||
        observed == null ||
        configuration == null ||
        installation == null ||
        !installation.canStart) {
      return;
    }
    final random = Random.secure();
    final operationId = List.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
    ).join();
    final categories = List<LibraryClassificationCategory>.unmodifiable(
      _categories,
    );
    final operation = installation.start(
      (commit) => configuration.enableManagedInstallation(
        operationId: operationId,
        observed: observed,
        commitInstallation: commit,
        editedCategories: categories,
      ),
    );
    Navigator.of(context).pop();
    unawaited(operation);
  }

  @override
  Widget build(BuildContext context) {
    final installation = ref.watch(libraryModelInstallationProvider);
    final installing = installation?.busy ?? false;
    final needsInstallation = _observed?.installationSelected != true;
    Widget content;
    if (_observed == null) {
      content = Center(
        child: _busy
            ? const CircularProgressIndicator()
            : Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(_error ?? "无法读取智能相册设置"),
                  TextButton(onPressed: _load, child: const Text("重新加载设置")),
                ],
              ),
      );
    } else if (_categories.isEmpty) {
      content = const Text("启用后会在本机分析图片，并将分类结果显示在智能相册中。首次使用需要下载模型（约 504 MB）。");
    } else {
      content = _categoryList(installing, needsInstallation);
    }
    return PopScope(
      canPop: !_busy || _observed == null,
      child: AlertDialog(
        key: const Key("classification-settings-editor"),
        title: const Text("智能相册设置"),
        content: SizedBox(
          width: 600,
          height: _observed == null || _categories.isNotEmpty ? 420 : null,
          child: content,
        ),
        actions: [
          if (_error != null && _observed != null)
            TextButton(
              onPressed: _busy ? null : _load,
              child: const Text("重新加载设置"),
            ),
          TextButton(
            onPressed: _busy && _observed != null
                ? null
                : () => Navigator.of(context).pop(),
            child: const Text("取消"),
          ),
          if (installing)
            TextButton(
              onPressed: installation?.canCancel == true
                  ? installation?.cancel
                  : null,
              child: const Text("取消启用"),
            )
          else if (_observed != null &&
              (needsInstallation || _categories.isEmpty))
            FilledButton(
              onPressed: _busy || installation?.canStart != true
                  ? null
                  : _install,
              child: const Text("下载并启用"),
            )
          else if (_observed?.mode == LibraryClassificationMode.disabled ||
              _observed?.mode == LibraryClassificationMode.stopped)
            TextButton(
              onPressed: _busy || installing ? null : () => _save(enable: true),
              child: const Text("启用"),
            ),
          if (_categories.isNotEmpty)
            FilledButton(
              onPressed: _busy || installing || _observed == null
                  ? null
                  : () => _save(enable: false),
              child: const Text("保存"),
            ),
        ],
      ),
    );
  }

  Widget _categoryList(bool installing, bool needsInstallation) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Row(
        children: [
          Expanded(
            child: Text("分类", style: Theme.of(context).textTheme.titleMedium),
          ),
          TextButton.icon(
            onPressed: _busy || installing || _categories.length >= 256
                ? null
                : () => _edit(),
            icon: const Icon(Symbols.add_rounded),
            label: const Text("新建分类"),
          ),
        ],
      ),
      Expanded(
        child: ListView.builder(
          itemCount: _categories.length,
          itemBuilder: (context, index) => ListTile(
            title: Text(_categories[index].label),
            subtitle: Text(
              _categories[index].descriptions.first,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            onTap: _busy || installing ? null : () => _edit(index),
            trailing: IconButton(
              tooltip: "移除分类",
              icon: const Icon(Symbols.delete_rounded),
              onPressed: _busy || installing || _categories.length <= 1
                  ? null
                  : () => setState(() {
                      _categories.removeAt(index);
                      _operationId = null;
                    }),
            ),
          ),
        ),
      ),
      if (needsInstallation) const Text("首次启用需要下载模型（约 504 MB），图片在本机分析。"),
      if (_error != null)
        Text(
          _error!,
          style: TextStyle(color: Theme.of(context).colorScheme.error),
        ),
    ],
  );
}
