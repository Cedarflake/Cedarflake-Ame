import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_similar_analysis.dart";
import "../../features/library/application/library_foreground_reads.dart";
import "../../features/library/application/library_similar_analysis_controller.dart";
import "../../features/library/application/library_similar_analysis_scheduler.dart";
import "../../features/library/application/ports/library_similar_analysis.dart";

class LibrarySimilarAnalysisBindings {
  LibrarySimilarAnalysisBindings({
    required this.service,
    required LibraryForegroundReads foregroundReads,
  }) {
    analysis = LibrarySimilarAnalysisController(service);
    scheduler = LibrarySimilarAnalysisScheduler(
      analysis,
      foregroundReads: foregroundReads,
    );
  }

  LibrarySimilarAnalysisBindings.production({
    required LibraryForegroundReads foregroundReads,
  }) : this(service: productionService, foregroundReads: foregroundReads);

  static const LibrarySimilarAnalysis productionService =
      RustLibrarySimilarAnalysis();

  final LibrarySimilarAnalysis service;
  late final LibrarySimilarAnalysisController analysis;
  late final LibrarySimilarAnalysisScheduler scheduler;
  Future<void>? _closing;

  List<Override> get overrides => [
    librarySimilarAnalysisProvider.overrideWithValue(service),
    librarySimilarAnalysisControllerProvider.overrideWithValue(analysis),
    librarySimilarAnalysisSchedulerProvider.overrideWithValue(scheduler),
  ];

  Future<void> close() => _closing ??= _close();
  Future<void> _close() async {
    await scheduler.close();
    scheduler.dispose();
    analysis.dispose();
  }
}

final librarySimilarAnalysisRequirements = <ProviderListenable<Object?>>[
  librarySimilarAnalysisProvider,
  librarySimilarAnalysisControllerProvider,
  librarySimilarAnalysisSchedulerProvider,
];
