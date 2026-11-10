import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../library/application/library_classification_configuration.dart";
import "../../../library/domain/library_classification_configuration.dart";
import "settings_section.dart";

class AnalysisSettingsSection extends ConsumerStatefulWidget {
  const AnalysisSettingsSection({super.key});

  @override
  ConsumerState<AnalysisSettingsSection> createState() =>
      _AnalysisSettingsSectionState();
}

class _AnalysisSettingsSectionState
    extends ConsumerState<AnalysisSettingsSection> {
  LibraryClassificationAcceleration? _settings;
  bool _saving = false;
  bool _loading = false;
  bool _loadFailed = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    if (!mounted || _loading) return;
    setState(() => _loading = true);
    try {
      final controller = ref.read(libraryClassificationConfigurationProvider);
      if (controller == null) return;
      final settings = await controller.loadAcceleration();
      if (mounted) {
        setState(() {
          _settings = settings;
          _loadFailed = false;
        });
      }
    } on Object {
      if (mounted) setState(() => _loadFailed = true);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _save(bool enabled) async {
    final controller = ref.read(libraryClassificationConfigurationProvider);
    if (controller == null || _saving) return;
    setState(() => _saving = true);
    try {
      final settings = await controller.changeAcceleration(gpuEnabled: enabled);
      if (mounted) setState(() => _settings = settings);
    } on Object catch (error) {
      // The preference can be committed even when reopening the worker fails.
      await _load();
      if (!mounted) return;
      ref
          .read(ameNotificationControllerProvider.notifier)
          .publish(
            AmeNotificationDraft(
              title: "GPU 加速设置未能完成",
              message: "请重试；如果仍无法分析图片，可以关闭 GPU 加速。",
              detail: error.toString(),
              severity: AmeNotificationSeverity.error,
              isPersistent: true,
            ),
          );
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = _settings;
    return SettingsSection(
      title: "智能相册",
      children: [
        SettingsRow(
          icon: Symbols.memory_rounded,
          title: "GPU 加速",
          subtitle: Text(_description),
          trailing: _loadFailed
              ? FilledButton.tonal(
                  onPressed: _loading ? null : _load,
                  child: const Text("重试"),
                )
              : Semantics(
                  label: "GPU 加速",
                  child: Switch(
                    key: const Key("settings-gpu-acceleration"),
                    value: settings?.gpuEnabled ?? true,
                    onChanged: settings == null || _saving || _loading
                        ? null
                        : _save,
                  ),
                ),
        ),
      ],
    );
  }

  String get _description {
    if (_loading) return "正在读取设置";
    if (_loadFailed) return "无法读取设置，请重试";
    if (_saving) return "正在切换分析设备";
    final settings = _settings;
    if (settings == null) return "正在读取设置";
    if (!settings.gpuEnabled) return "智能相册当前使用 CPU 分析图片";
    final deviceName = settings.deviceName;
    if (deviceName != null) return "智能相册当前图片分析设备：$deviceName";
    return "智能相册优先使用可用显卡分析图片";
  }
}
