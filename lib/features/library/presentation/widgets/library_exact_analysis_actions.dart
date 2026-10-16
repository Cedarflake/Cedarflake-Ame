import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

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
    if (controller.hasFailure || (progress?.skippedFiles ?? 0) > 0) {
      actionLabel = LibraryDuplicateStrings.retry;
    } else if (progress == null) {
      actionLabel = LibraryDuplicateStrings.start;
    } else {
      actionLabel = LibraryDuplicateStrings.refreshAnalysis;
    }
    return Wrap(
      key: const Key("duplicate-review-actions"),
      spacing: 8,
      runSpacing: 4,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        if (scheduler != null) ...[
          if (controller.hasFailure || (progress?.skippedFiles ?? 0) > 0)
            TextButton(
              onPressed: scheduler.isPaused || controller.isRunning
                  ? null
                  : scheduler.requestNow,
              child: const Text(LibraryDuplicateStrings.retry),
            ),
          TextButton(
            onPressed: controller.isStopping
                ? null
                : () =>
                      unawaited(_setPaused(context, ref, !scheduler.isPaused)),
            child: Text(
              scheduler.isPaused
                  ? LibraryDuplicateStrings.resume
                  : LibraryDuplicateStrings.pause,
            ),
          ),
        ] else if (controller.isRunning)
          TextButton(
            onPressed: controller.isStopping
                ? null
                : () => unawaited(controller.stop()),
            child: const Text(LibraryDuplicateStrings.stop),
          )
        else
          FilledButton(onPressed: controller.start, child: Text(actionLabel)),
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
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text(LibraryDuplicateStrings.pauseSaveFailed),
          ),
        );
      }
    }
  }
}
