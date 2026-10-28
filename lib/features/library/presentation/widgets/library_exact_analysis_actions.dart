import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../library_duplicate_strings.dart";

class LibraryExactAnalysisActions extends ConsumerWidget {
  const LibraryExactAnalysisActions({
    required this.controller,
    this.actions = const [],
    super.key,
  });
  final LibraryExactAnalysisController controller;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scheduler = ref.watch(libraryExactAnalysisSchedulerProvider);
    final progress = controller.progress;
    final String actionLabel;
    final IconData actionIcon;
    if (controller.hasFailure || (progress?.skippedFiles ?? 0) > 0) {
      actionLabel = LibraryDuplicateStrings.retry;
      actionIcon = Symbols.refresh_rounded;
    } else if (progress == null) {
      actionLabel = LibraryDuplicateStrings.start;
      actionIcon = Symbols.search_rounded;
    } else {
      actionLabel = LibraryDuplicateStrings.refreshAnalysis;
      actionIcon = Symbols.refresh_rounded;
    }
    return Wrap(
      key: const Key("duplicate-review-actions"),
      alignment: WrapAlignment.end,
      spacing: 8,
      runSpacing: 4,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        if (scheduler != null) ...[
          if (controller.hasFailure || (progress?.skippedFiles ?? 0) > 0)
            TextButton.icon(
              onPressed: scheduler.isPaused || controller.isRunning
                  ? null
                  : scheduler.requestNow,
              icon: const Icon(Symbols.refresh_rounded),
              label: const Text(LibraryDuplicateStrings.retry),
            ),
          TextButton.icon(
            onPressed: controller.isStopping
                ? null
                : () =>
                      unawaited(_setPaused(context, ref, !scheduler.isPaused)),
            icon: Icon(
              scheduler.isPaused
                  ? Symbols.play_arrow_rounded
                  : Symbols.pause_rounded,
            ),
            label: Text(
              scheduler.isPaused
                  ? LibraryDuplicateStrings.resume
                  : LibraryDuplicateStrings.pause,
            ),
          ),
        ] else if (controller.isRunning)
          TextButton.icon(
            onPressed: controller.isStopping
                ? null
                : () => unawaited(controller.stop()),
            icon: const Icon(Symbols.stop_rounded),
            label: const Text(LibraryDuplicateStrings.stop),
          )
        else
          FilledButton.icon(
            onPressed: controller.start,
            icon: Icon(actionIcon),
            label: Text(actionLabel),
          ),
        ...actions,
      ],
    );
  }

  Future<void> _setPaused(
    BuildContext context,
    WidgetRef ref,
    bool paused,
  ) async {
    final preferences = ref.read(amePreferencesControllerProvider);
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences.copyWith(exactAnalysisPaused: paused));
    } on Object {
      if (context.mounted) {
        showAmeTransientFeedback(
          context,
          message: LibraryDuplicateStrings.pauseSaveFailed,
          severity: AmeNotificationSeverity.error,
        );
      }
    }
  }
}
