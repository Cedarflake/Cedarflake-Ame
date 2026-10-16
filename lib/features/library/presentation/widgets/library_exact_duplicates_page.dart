import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_results_controller.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../application/library_exact_duplicates.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_asset_summary.dart";
import "../library_duplicate_strings.dart";
import "library_path_text.dart";
import "../library_workbench_strings.dart";
import "library_duplicate_location_tile.dart";

class LibraryExactDuplicatesPage extends ConsumerStatefulWidget {
  const LibraryExactDuplicatesPage({super.key});

  @override
  ConsumerState<LibraryExactDuplicatesPage> createState() =>
      _LibraryExactDuplicatesPageState();
}

class _LibraryExactDuplicatesPageState
    extends ConsumerState<LibraryExactDuplicatesPage> {
  late final LibraryExactAnalysisController _analysis;
  late final LibraryDuplicateResultsController _results;
  LibraryExactAnalysisScheduler? _scheduler;
  late bool _wasRunning;

  @override
  void initState() {
    super.initState();
    _analysis = ref.read(libraryExactAnalysisControllerProvider);
    _scheduler = ref.read(libraryExactAnalysisSchedulerProvider);
    _results = LibraryDuplicateResultsController(
      ref.read(libraryExactDuplicatesProvider),
    );
    _wasRunning = _analysis.isRunning;
    _analysis.addListener(_analysisChanged);
    ref.listenManual(
      libraryControllerProvider.select((state) => state.catalogRevision),
      (_, _) => unawaited(_results.catalogChanged()),
    );
    unawaited(_results.refresh());
  }

  void _analysisChanged() {
    if (_wasRunning && !_analysis.isRunning) {
      unawaited(_results.refresh());
    }
    _wasRunning = _analysis.isRunning;
  }

  @override
  void dispose() {
    _analysis.removeListener(_analysisChanged);
    _results.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: Listenable.merge([_analysis, _results, _scheduler]),
      builder: (context, child) => Padding(
        padding: const EdgeInsets.fromLTRB(28, 24, 28, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                if (_results.selected != null)
                  AmeTooltip(
                    message: LibraryDuplicateStrings.back,
                    child: IconButton(
                      onPressed: () => unawaited(_results.showGroups()),
                      icon: const Icon(Symbols.arrow_back_rounded),
                    ),
                  ),
                Expanded(
                  child: Text(
                    _results.selected == null
                        ? LibraryWorkbenchStrings.duplicates
                        : LibraryDuplicateStrings.viewLocations,
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            const Text(LibraryDuplicateStrings.exactDescription),
            const SizedBox(height: 20),
            _AnalysisStatus(controller: _analysis),
            const SizedBox(height: 12),
            if (_results.loading) const LinearProgressIndicator(),
            Expanded(
              child: _results.hasError
                  ? Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          const Text(LibraryDuplicateStrings.loadFailed),
                          TextButton(
                            onPressed: () => unawaited(_results.refresh()),
                            child: const Text(LibraryDuplicateStrings.retry),
                          ),
                        ],
                      ),
                    )
                  : _buildResults(),
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: _results.loading
                      ? null
                      : () => unawaited(_results.refresh()),
                  child: const Text(LibraryDuplicateStrings.refresh),
                ),
                const Spacer(),
                TextButton(
                  onPressed: !_results.canPrevious || _results.loading
                      ? null
                      : () => unawaited(_results.previous()),
                  child: const Text(LibraryDuplicateStrings.previous),
                ),
                TextButton(
                  onPressed: !_results.canNext || _results.loading
                      ? null
                      : () => unawaited(_results.next()),
                  child: const Text(LibraryDuplicateStrings.next),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildResults() {
    final selected = _results.selected;
    if (selected != null) {
      if (_results.locations.locations.isEmpty) {
        return Center(
          child: Text(
            _results.loading ? "" : LibraryDuplicateStrings.emptyLocations,
          ),
        );
      }
      return AmeScrollMotion(
        child: ListView.builder(
          key: ValueKey("locations-${selected.digest}"),
          itemCount: _results.locations.locations.length,
          itemBuilder: (context, index) {
            final location = _results.locations.locations[index];
            return LibraryDuplicateLocationTile(location: location);
          },
        ),
      );
    }
    if (_results.groups.groups.isEmpty) {
      String message;
      if (_results.loading) {
        message = "";
      } else if (_analysis.isRunning) {
        message = LibraryDuplicateStrings.waitingForResults;
      } else if (_analysis.progress == null) {
        message = LibraryDuplicateStrings.emptyBeforeAnalysis;
      } else {
        message = LibraryDuplicateStrings.emptyAfterAnalysis;
      }
      return Center(child: Text(message));
    }
    return AmeScrollMotion(
      child: ListView.builder(
        itemCount: _results.groups.groups.length,
        itemBuilder: (context, index) {
          final group = _results.groups.groups[index];
          return ListTile(
            onTap: () => unawaited(_results.showLocations(group)),
            leading: const Icon(Symbols.photo_library_rounded),
            title: Text(
              displayLibraryFileName(group.representativePath),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            subtitle: Text(
              [
                LibraryDuplicateStrings.locations(group.locationCount),
                if (group.physicalFileCount < group.locationCount)
                  LibraryDuplicateStrings.physicalFiles(
                    group.physicalFileCount,
                  ),
                libraryFileSize(BigInt.from(group.byteLength)),
              ].join(" · "),
            ),
            trailing: TextButton(
              onPressed: () => unawaited(_results.showLocations(group)),
              child: const Text(LibraryDuplicateStrings.viewLocations),
            ),
          );
        },
      ),
    );
  }
}

class _AnalysisStatus extends ConsumerWidget {
  const _AnalysisStatus({required this.controller});
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
