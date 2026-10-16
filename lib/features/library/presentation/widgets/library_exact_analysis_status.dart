import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";

class LibraryExactAnalysisStatus extends ConsumerWidget {
  const LibraryExactAnalysisStatus({required this.controller, super.key});
  final LibraryExactAnalysisController controller;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scheduler = ref.watch(libraryExactAnalysisSchedulerProvider);
    final progress = controller.progress;
    final String title;
    if (controller.isStopping) {
      title = LibraryDuplicateStrings.stopping;
    } else if (controller.hasFailure) {
      title = LibraryDuplicateStrings.failed;
    } else if (scheduler?.isPaused ?? false) {
      title = LibraryDuplicateStrings.paused;
    } else if (scheduler?.isWaiting ?? false) {
      title = LibraryDuplicateStrings.automaticWaiting;
    } else {
      title = switch (progress?.phase) {
        LibraryExactAnalysisPhase.running => LibraryDuplicateStrings.running,
        LibraryExactAnalysisPhase.completed =>
          LibraryDuplicateStrings.completed,
        LibraryExactAnalysisPhase.cancelled =>
          LibraryDuplicateStrings.cancelled,
        LibraryExactAnalysisPhase.failed => LibraryDuplicateStrings.failed,
        null => "",
      };
    }
    final String actionLabel;
    if (controller.hasFailure || (progress?.skippedFiles ?? 0) > 0) {
      actionLabel = LibraryDuplicateStrings.retry;
    } else if (progress == null) {
      actionLabel = LibraryDuplicateStrings.start;
    } else {
      actionLabel = LibraryDuplicateStrings.refreshAnalysis;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            if (controller.isRunning) ...[
              const SizedBox(
                width: 18,
                height: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
              const SizedBox(width: 12),
            ],
            Expanded(child: Semantics(liveRegion: true, child: Text(title))),
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
                    : () => unawaited(
                        _setPaused(context, ref, !scheduler.isPaused),
                      ),
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
              FilledButton(
                onPressed: controller.start,
                child: Text(actionLabel),
              ),
          ],
        ),
        if (progress != null && progress.checkedFiles > 0)
          Text(LibraryDuplicateStrings.checked(progress.checkedFiles)),
        if ((progress?.skippedFiles ?? 0) > 0)
          Text(LibraryDuplicateStrings.skipped(progress!.skippedFiles)),
        if (progress?.currentPath case final String path
            when controller.isRunning)
          Text(
            path,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodySmall,
          ),
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
