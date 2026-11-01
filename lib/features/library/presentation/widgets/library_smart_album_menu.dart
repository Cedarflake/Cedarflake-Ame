import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_settings.dart";
import "library_classification_settings_editor.dart";

enum _Action { selectAll, settings, pause, resume, stop, disable }

class LibrarySmartAlbumMenu extends ConsumerStatefulWidget {
  const LibrarySmartAlbumMenu({required this.onSelectAll, super.key});

  final VoidCallback onSelectAll;

  @override
  ConsumerState<LibrarySmartAlbumMenu> createState() =>
      _LibrarySmartAlbumMenuState();
}

class _LibrarySmartAlbumMenuState extends ConsumerState<LibrarySmartAlbumMenu> {
  bool _busy = false;

  Future<void> _apply(
    _Action action,
    LibraryClassificationSettings? observed,
  ) async {
    if (_busy) return;
    if (action == _Action.selectAll) {
      widget.onSelectAll();
      return;
    }
    if (action == _Action.settings) {
      await showLibraryClassificationSettings(context);
      return;
    }
    final service = ref.read(libraryClassificationConfigurationProvider);
    if (observed == null || service == null) return;
    setState(() => _busy = true);
    try {
      await service.changeControl(
        expectedRevision: observed.dispatchRevision,
        action: switch (action) {
          _Action.pause => LibraryClassificationControl.pause,
          _Action.resume => LibraryClassificationControl.resume,
          _Action.stop => LibraryClassificationControl.stop,
          _Action.disable => LibraryClassificationControl.disable,
          _Action.settings || _Action.selectAll => throw StateError(
            "Settings have no dispatch action",
          ),
        },
      );
      if (!mounted) return;
      ref.invalidate(libraryClassificationSettingsProvider);
      showAmeTransientFeedback(
        context,
        severity: AmeNotificationSeverity.success,
        message: switch (action) {
          _Action.pause => "已暂停分析",
          _Action.resume => "已继续分析",
          _Action.stop => "已停止分析",
          _Action.disable => "已关闭后台分析",
          _Action.settings || _Action.selectAll => throw StateError(
            "Settings have no dispatch feedback",
          ),
        },
      );
    } on Object {
      if (mounted) {
        ref.invalidate(libraryClassificationSettingsProvider);
        showAmeTransientFeedback(
          context,
          message: "未能更新分析状态，请重试。",
          severity: AmeNotificationSeverity.error,
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref
        .watch(libraryClassificationSettingsProvider)
        .asData
        ?.value;
    final actions = <_Action, (String, IconData)>{
      _Action.selectAll: ("全选", Symbols.select_all_rounded),
      _Action.settings: ("智能相册设置", Symbols.settings_rounded),
      if (settings?.mode == LibraryClassificationMode.automatic)
        _Action.pause: ("暂停分析", Symbols.pause_rounded),
      if (settings?.mode == LibraryClassificationMode.paused)
        _Action.resume: ("继续分析", Symbols.play_arrow_rounded),
      if (settings?.mode == LibraryClassificationMode.automatic ||
          settings?.mode == LibraryClassificationMode.paused)
        _Action.stop: ("停止分析", Symbols.stop_rounded),
      if (settings != null &&
          settings.mode != LibraryClassificationMode.disabled)
        _Action.disable: ("关闭后台分析", Symbols.power_settings_new_rounded),
    };
    return AmePopupMenuButton<_Action>(
      labels: actions.values.map((entry) => entry.$1).toList(),
      shortcuts: const ["Ctrl+A"],
      viewportRightMargin: AmeMenuMetrics.viewportPadding,
      items: [
        for (final entry in actions.entries)
          PopupMenuItem(
            value: entry.key,
            child: AmeMenuItemContent(
              icon: entry.value.$2,
              label: entry.value.$1,
              shortcut: entry.key == _Action.selectAll ? "Ctrl+A" : null,
            ),
          ),
      ],
      onSelected: (action) => _apply(action, settings),
      builder: (context, openMenu) => AmeTooltip(
        message: "更多",
        child: IconButton(
          key: const Key("library-more-menu"),
          onPressed: _busy ? null : openMenu,
          icon: const Icon(Symbols.more_horiz_rounded),
        ),
      ),
    );
  }
}
