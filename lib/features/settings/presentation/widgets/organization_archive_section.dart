import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/application/library_archive_controller.dart";
import "../../../library/application/library_archive_requests.dart";
import "../../../library/application/library_archive_state.dart";
import "../../application/organization_archive_picker.dart";
import "../organization_archive_messages.dart";
import "organization_archive_confirmation.dart";
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
  String? _pickerError;

  Future<void> _choose({required bool exporting}) async {
    if (_picking ||
        ref.read(libraryArchiveControllerProvider) is LibraryArchiveWorking) {
      return;
    }
    setState(() {
      _picking = true;
      _pickerError = null;
    });
    String? path;
    try {
      final picker = ref.read(organizationArchivePickerProvider);
      path = await (exporting ? picker.destination() : picker.backup());
    } on Object {
      if (mounted) setState(() => _pickerError = "无法打开文件选择窗口，请重试。");
    } finally {
      if (mounted) setState(() => _picking = false);
    }
    if (!mounted || path == null) return;
    final controller = ref.read(libraryArchiveControllerProvider.notifier);
    if (exporting) {
      await controller.export(path);
    } else {
      await controller.inspect(path);
      if (mounted) await _confirm();
    }
  }

  Future<void> _confirm() async {
    final ready = ref.read(libraryArchiveControllerProvider);
    if (ready is! LibraryArchiveReady) return;
    final confirmed = await showOrganizationArchiveConfirmation(context, ready);
    if (!mounted) return;
    final controller = ref.read(libraryArchiveControllerProvider.notifier);
    if (confirmed) {
      await controller.restore(ready);
    } else if (identical(ref.read(libraryArchiveControllerProvider), ready)) {
      controller.dismiss();
    }
  }

  Future<void> _retry() async {
    await ref.read(libraryArchiveControllerProvider.notifier).retry();
    if (mounted) await _confirm();
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
            onPressed: busy ? null : () => unawaited(_choose(exporting: false)),
            child: const Text("选择文件"),
          ),
        ),
        if (_pickerError case final message?)
          Padding(
            padding: const EdgeInsets.all(20),
            child: Text(
              message,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        if (state is! LibraryArchiveIdle)
          Padding(padding: const EdgeInsets.all(20), child: _feedback(state)),
        const OrganizationCleanupRow(),
      ],
    );
  }

  Widget _feedback(LibraryArchiveState state) {
    final controller = ref.read(libraryArchiveControllerProvider.notifier);
    return switch (state) {
      LibraryArchiveWorking() => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Semantics(
            liveRegion: true,
            child: Text(archiveProgressMessage(state)),
          ),
          const SizedBox(height: 12),
          const LinearProgressIndicator(),
          if (state.cancelFailed)
            const Padding(
              padding: EdgeInsets.only(top: 12),
              child: Text("尚未停止，请再次尝试。"),
            ),
          const SizedBox(height: 8),
          TextButton(
            key: const Key("settings-archive-cancel"),
            onPressed: state.stopping ? null : controller.cancel,
            child: const Text("取消"),
          ),
        ],
      ),
      LibraryArchiveReady() => Wrap(
        spacing: 12,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          Text("已检查 ${archiveFilename(state.path)}"),
          TextButton(
            onPressed: () => unawaited(_confirm()),
            child: const Text("查看备份"),
          ),
          TextButton(onPressed: controller.dismiss, child: const Text("取消")),
        ],
      ),
      LibraryArchiveFailed() => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Semantics(
            liveRegion: true,
            child: Text(
              archiveFailureMessage(state.failure),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: [
              TextButton(
                key: const Key("settings-archive-retry"),
                onPressed: () => unawaited(_retry()),
                child: Text(state.failure.requiresInspection ? "重新检查" : "重试"),
              ),
              TextButton(
                onPressed: controller.dismiss,
                child: const Text("关闭"),
              ),
            ],
          ),
        ],
      ),
      LibraryArchiveFinished() => Wrap(
        spacing: 12,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          Text(
            state.cancelled
                ? "已停止${archiveOperationName(state.request)}"
                : switch (state.request) {
                    LibraryArchiveExport() =>
                      "已导出 ${archiveFilename(state.request.path)}",
                    LibraryArchiveRestore() => "整理记录已恢复",
                    LibraryArchiveInspect() => "备份检查已结束",
                  },
          ),
          TextButton(onPressed: controller.dismiss, child: const Text("关闭")),
        ],
      ),
      LibraryArchiveIdle() => const SizedBox.shrink(),
    };
  }
}
