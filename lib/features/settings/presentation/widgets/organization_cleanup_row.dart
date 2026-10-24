import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../../../library/domain/library_organization_cleanup.dart";
import "organization_cleanup_confirmation.dart";
import "settings_section.dart";

class OrganizationCleanupRow extends ConsumerStatefulWidget {
  const OrganizationCleanupRow({super.key});
  @override
  ConsumerState<OrganizationCleanupRow> createState() =>
      _OrganizationCleanupRowState();
}

class _OrganizationCleanupRowState
    extends ConsumerState<OrganizationCleanupRow> {
  int _minimumAgeDays = 90;

  Future<void> _inspect() async {
    await ref
        .read(libraryCleanupControllerProvider.notifier)
        .inspect(_minimumAgeDays == 0 ? null : _minimumAgeDays);
    if (mounted) await _confirm();
  }

  Future<void> _confirm() async {
    final ready = ref.read(libraryCleanupControllerProvider);
    if (ready is! LibraryCleanupReady || ready.inspection.total == 0) {
      return;
    }
    final confirmed = await showOrganizationCleanupConfirmation(
      context,
      ready.inspection,
    );
    if (mounted && confirmed) {
      await ref.read(libraryCleanupControllerProvider.notifier).apply(ready);
    }
  }

  Future<void> _retry() async {
    await ref.read(libraryCleanupControllerProvider.notifier).retry();
    if (mounted) await _confirm();
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryCleanupControllerProvider);
    final busy = state is LibraryCleanupWorking;
    final selectedAge = switch (state) {
      LibraryCleanupReady(:final minimumAgeDays) => minimumAgeDays ?? 0,
      LibraryCleanupWorking(:final request) ||
      LibraryCleanupFailed(:final request) => request.minimumAgeDays ?? 0,
      _ => _minimumAgeDays,
    };
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SettingsRow(
          icon: Symbols.cleaning_services_rounded,
          title: "清理未关联记录",
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text("检查无法关联图片的相册、收藏和整理记录"),
              const SizedBox(height: 12),
              SettingsChoice<int>(
                key: const Key("settings-cleanup-age"),
                value: selectedAge,
                width: 200,
                enabled: !busy,
                entries: const [
                  SettingsChoiceEntry(value: 90, label: "未关联超过 90 天"),
                  SettingsChoiceEntry(value: 30, label: "未关联超过 30 天"),
                  SettingsChoiceEntry(value: 0, label: "全部未关联记录"),
                ],
                onSelected: (value) {
                  if (value == null) return;
                  ref.read(libraryCleanupControllerProvider.notifier).dismiss();
                  setState(() => _minimumAgeDays = value);
                },
              ),
            ],
          ),
          trailing: FilledButton.tonal(
            key: const Key("settings-cleanup-inspect"),
            onPressed: busy
                ? null
                : () {
                    _minimumAgeDays = selectedAge;
                    unawaited(_inspect());
                  },
            child: const Text("检查"),
          ),
        ),
        if (state is! LibraryCleanupIdle)
          Padding(padding: const EdgeInsets.all(20), child: _feedback(state)),
      ],
    );
  }

  Widget _feedback(LibraryCleanupState state) {
    final controller = ref.read(libraryCleanupControllerProvider.notifier);
    return switch (state) {
      LibraryCleanupWorking() => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Semantics(liveRegion: true, child: Text(_progressMessage(state))),
          const SizedBox(height: 12),
          const LinearProgressIndicator(),
          if (state.cancelFailed) const Text("尚未停止，请再次尝试。"),
          TextButton(
            key: const Key("settings-cleanup-cancel"),
            onPressed: state.stopping ? null : controller.cancel,
            child: const Text("取消"),
          ),
        ],
      ),
      LibraryCleanupReady(:final inspection) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            inspection.total == 0
                ? "没有符合条件的未关联记录"
                : "${inspection.total} 项记录可以清理",
          ),
          if (inspection.total > 0)
            TextButton(
              onPressed: () => unawaited(_confirm()),
              child: const Text("查看并清理"),
            ),
        ],
      ),
      LibraryCleanupFailed() => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_failureMessage(state)),
          TextButton(
            key: const Key("settings-cleanup-retry"),
            onPressed: () => unawaited(_retry()),
            child: Text(state.failure.requiresInspection ? "重新检查" : "重试"),
          ),
        ],
      ),
      LibraryCleanupFinished(:final cancelled, :final removedRecords) => Text(
        cancelled ? "已取消" : "已清理 $removedRecords 项记录",
      ),
      LibraryCleanupIdle() => const SizedBox.shrink(),
    };
  }

  String _progressMessage(LibraryCleanupWorking state) {
    if (state.stopping) return "正在停止";
    return switch (state.phase) {
      LibraryCleanupPhase.inspecting => "正在检查未关联记录",
      LibraryCleanupPhase.applying => "正在清理记录",
    };
  }

  String _failureMessage(LibraryCleanupFailed state) {
    if (state.failure.requiresInspection) return "记录已发生变化，请重新检查后确认。";
    if (state.request is LibraryCleanupInspect) return "未能完成检查，请重试。";
    return "未能确认清理结果，请重试。";
  }
}
