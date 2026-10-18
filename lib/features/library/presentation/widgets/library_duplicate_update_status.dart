import "package:flutter/widgets.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../library_duplicate_strings.dart";
import "../library_strings.dart";
import "library_results_progress.dart";

/// Retained groups share one activity label without marking individual images.
class LibraryDuplicateUpdateStatus extends ConsumerWidget {
  const LibraryDuplicateUpdateStatus({
    required this.hasPending,
    required this.builder,
    super.key,
  });

  final bool hasPending;
  final Widget Function(BuildContext context, String? label) builder;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (!hasPending) return builder(context, null);
    final analysis = ref.watch(libraryExactAnalysisControllerProvider);
    final scheduler = ref.watch(libraryExactAnalysisSchedulerProvider);
    return ListenableBuilder(
      listenable: Listenable.merge([analysis, scheduler]),
      builder: (context, _) {
        final progress = LibraryResultsProgress.resolveAnalysis(
          analysis: analysis,
          scheduler: scheduler,
          hasPendingMembers: true,
        );
        final String? label;
        if (analysis.isStopping) {
          label = LibraryDuplicateStrings.stopping;
        } else if (progress == LibraryResultsProgress.analyzing) {
          label = LibraryStrings.synchronizing;
        } else {
          label = progress.label;
        }
        return builder(context, label);
      },
    );
  }
}
