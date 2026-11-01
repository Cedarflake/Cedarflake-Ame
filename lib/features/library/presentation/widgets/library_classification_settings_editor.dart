import "dart:async";
import "dart:math";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_lifecycle.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_model_installation.dart";
import "../library_classification_notifications.dart";

Future<void> showLibraryClassificationSettings(BuildContext context) async {
  final enabled = await showDialog<bool>(
    context: context,
    builder: (context) => const _SettingsEditor(),
  );
  if (context.mounted && enabled == true) {
    showAmeTransientFeedback(
      context,
      message: "已启用智能相册",
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
  String? _error;
  String? _operationId;
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
          _operationId = null;
        });
      }
    } on Object {
      if (mounted) setState(() => _error = "无法读取智能相册设置，请重试。");
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _enable() async {
    final observed = _observed;
    final service = ref.read(libraryClassificationConfigurationProvider);
    if (_busy || observed == null || service == null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    final random = Random.secure();
    _operationId ??= List.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
    ).join();
    try {
      await service.enable(operationId: _operationId!, observed: observed);
      if (!mounted) return;
      ref.invalidate(libraryClassificationSettingsProvider);
      Navigator.of(context).pop(true);
    } on Object {
      if (mounted) setState(() => _error = "未能启用智能相册，请重新加载设置后重试。");
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
    final operation = installation.start(
      (commit) => configuration.enableManagedInstallation(
        operationId: operationId,
        observed: observed,
        commitInstallation: commit,
      ),
    );
    Navigator.of(context).pop();
    unawaited(operation);
  }

  @override
  Widget build(BuildContext context) {
    final analysisError = ref.watch(
      libraryClassificationLifecycleProvider.select((owner) => owner?.error),
    );
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
    } else if (needsInstallation) {
      content = const Text("启用后会在本机分析图片，并将分类结果显示在智能相册中。首次使用需要下载模型（约 504 MB）。");
    } else if (analysisError != null) {
      content = const Text(
        "$libraryClassificationFailureTitle，$libraryClassificationFailureMessage",
      );
    } else {
      content = Text(switch (_observed!.mode) {
        LibraryClassificationMode.disabled => "启用后会在本机分析图片，并根据图库内容自动生成和调整分类。",
        LibraryClassificationMode.automatic => "图片在本机分析，分类会根据图库内容自动生成和调整。",
        LibraryClassificationMode.paused => "分析已暂停，已有分类结果仍可浏览。",
        LibraryClassificationMode.stopped => "分析已停止，已有分类结果仍可浏览。",
      });
    }
    return PopScope(
      canPop: !_busy || _observed == null,
      child: AlertDialog(
        key: const Key("classification-settings-editor"),
        title: const Text("智能相册设置"),
        content: SizedBox(
          width: 480,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              content,
              if (_error != null && _observed != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
            ],
          ),
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
            child: const Text("关闭"),
          ),
          if (installing)
            TextButton(
              onPressed: installation?.canCancel == true
                  ? installation?.cancel
                  : null,
              child: const Text("取消启用"),
            )
          else if (_observed != null && needsInstallation)
            FilledButton(
              onPressed: _busy || installation?.canStart != true
                  ? null
                  : _install,
              child: const Text("下载并启用"),
            )
          else if (_observed?.mode == LibraryClassificationMode.disabled ||
              _observed?.mode == LibraryClassificationMode.stopped)
            FilledButton(
              onPressed: _busy ? null : _enable,
              child: const Text("启用"),
            ),
        ],
      ),
    );
  }
}
