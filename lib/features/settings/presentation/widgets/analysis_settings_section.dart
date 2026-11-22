import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_localizations.dart";
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
              title: AmeLabel.gpuSettingFailed,
              message: AmeLabel.gpuSettingRetry,
              detail: AmeRawMessage(error.toString()),
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
      title: context.l10n.smartAlbums,
      children: [
        SettingsRow(
          icon: Symbols.memory_rounded,
          title: context.l10n.gpuAcceleration,
          subtitle: Text(_description),
          trailing: _loadFailed
              ? FilledButton.tonal(
                  onPressed: _loading ? null : _load,
                  child: Text(context.l10n.retry),
                )
              : Semantics(
                  label: context.l10n.gpuAcceleration,
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
    if (_loading) return context.l10n.loadingSettings;
    if (_loadFailed) return context.l10n.settingsReadFailed;
    if (_saving) return context.l10n.switchingAnalysisDevice;
    final settings = _settings;
    if (settings == null) return context.l10n.loadingSettings;
    if (!settings.gpuEnabled) return context.l10n.smartAlbumsCpu;
    final deviceName = settings.deviceName;
    if (deviceName != null) return context.l10n.smartAlbumsDevice(deviceName);
    return context.l10n.smartAlbumsPreferGpu;
  }
}
