import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../../../library/domain/library_organization_cleanup.dart";
import "organization_cleanup_confirmation.dart";
import "settings_section.dart";

class OrganizationCleanupRow extends ConsumerWidget {
  const OrganizationCleanupRow({super.key});

  Future<void> _inspect(BuildContext context, WidgetRef ref) async {
    final state = ref.read(libraryCleanupControllerProvider);
    final minimumAgeDays = switch (state) {
      LibraryCleanupReady(:final minimumAgeDays) => minimumAgeDays ?? 0,
      LibraryCleanupFailed(:final request) => request.minimumAgeDays ?? 0,
      _ => 90,
    };
    final confirmed = await showOrganizationCleanupConfirmation(
      context,
      minimumAgeDays: minimumAgeDays,
    );
    if (context.mounted && confirmed != null) {
      await ref
          .read(libraryCleanupControllerProvider.notifier)
          .apply(confirmed);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryCleanupControllerProvider);
    final busy = state is LibraryCleanupWorking;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SettingsRow(
          icon: Symbols.cleaning_services_rounded,
          title: "清理未关联记录",
          subtitle: const Text("检查无法关联图片的相册、收藏和整理记录"),
          trailing: FilledButton.tonal(
            key: const Key("settings-cleanup-inspect"),
            onPressed: busy ? null : () => unawaited(_inspect(context, ref)),
            child: const Text("查看并清理"),
          ),
        ),
        if (state is! LibraryCleanupIdle && state is! LibraryCleanupReady)
          Padding(
            padding: const EdgeInsets.all(20),
            child: _feedback(context, ref, state),
          ),
      ],
    );
  }

  Widget _feedback(
    BuildContext context,
    WidgetRef ref,
    LibraryCleanupState state,
  ) {
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
      LibraryCleanupFailed() => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_failureMessage(state)),
          TextButton(
            key: const Key("settings-cleanup-retry"),
            onPressed: () => unawaited(
              state.failure.requiresInspection ||
                      state.request is LibraryCleanupInspect
                  ? _inspect(context, ref)
                  : controller.retry(),
            ),
            child: Text(state.failure.requiresInspection ? "重新检查" : "重试"),
          ),
        ],
      ),
      LibraryCleanupFinished(:final cancelled, :final removedRecords) => Text(
        cancelled ? "已取消" : "已清理 $removedRecords 项记录",
      ),
      LibraryCleanupIdle() || LibraryCleanupReady() => const SizedBox.shrink(),
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
