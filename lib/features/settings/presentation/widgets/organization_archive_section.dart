import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../library/application/library_archive_controller.dart";
import "../../../library/application/library_archive_state.dart";
import "../../application/organization_archive_picker.dart";
import "../organization_record_actions.dart";
import "organization_cleanup_row.dart";
import "settings_section.dart";

class OrganizationArchiveSection extends ConsumerStatefulWidget {
  const OrganizationArchiveSection({super.key});

  @override
  ConsumerState<OrganizationArchiveSection> createState() =>
      _OrganizationArchiveSectionState();
}

class _OrganizationArchiveSectionState
    extends ConsumerState<OrganizationArchiveSection> {
  bool _picking = false;

  Future<void> _choose({required bool exporting}) async {
    if (_picking ||
        ref.read(libraryArchiveControllerProvider) is LibraryArchiveWorking) {
      return;
    }
    setState(() => _picking = true);
    String? path;
    try {
      final picker = ref.read(organizationArchivePickerProvider);
      path = await (exporting ? picker.destination() : picker.backup());
    } on Object {
      if (mounted) {
        ref
            .read(ameNotificationControllerProvider.notifier)
            .publish(
              const AmeNotificationDraft(
                title: "无法打开文件选择窗口",
                message: "请重试。",
                severity: AmeNotificationSeverity.error,
                dedupeKey: "organization-archive-picker",
              ),
            );
      }
    } finally {
      if (mounted) setState(() => _picking = false);
    }
    if (!mounted || path == null) return;
    final controller = ref.read(libraryArchiveControllerProvider.notifier);
    if (exporting) {
      await controller.export(path);
    } else {
      await controller.inspect(path);
      if (mounted) await confirmOrganizationArchive(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryArchiveControllerProvider);
    final busy = _picking || state is LibraryArchiveWorking;
    return SettingsSection(
      title: "整理记录",
      children: [
        SettingsRow(
          icon: Symbols.file_upload_rounded,
          title: "导出整理记录",
          subtitle: const Text("备份相册、收藏和整理进度，不包含原图片"),
          trailing: FilledButton.tonal(
            key: const Key("settings-archive-export"),
            onPressed: busy ? null : () => unawaited(_choose(exporting: true)),
            child: const Text("导出"),
          ),
        ),
        SettingsRow(
          icon: Symbols.restore_page_rounded,
          title: "恢复整理记录",
          subtitle: const Text("从备份恢复，保留当前已作出的选择"),
          trailing: FilledButton.tonal(
            key: const Key("settings-archive-inspect"),
            onPressed: busy
                ? null
                : () => unawaited(
                    state is LibraryArchiveReady
                        ? confirmOrganizationArchive(context)
                        : _choose(exporting: false),
                  ),
            child: Text(state is LibraryArchiveReady ? "查看备份" : "选择文件"),
          ),
        ),
        const OrganizationCleanupRow(),
      ],
    );
  }
}
