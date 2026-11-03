import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_duplicate_review.dart";
import "../../features/library/adapters/rust/rust_library_exact_duplicates.dart";
import "../../features/library/adapters/rust/rust_library_gallery_duplicates.dart";
import "../../features/library/application/library_duplicate_review.dart";
import "../../features/library/application/library_exact_analysis_controller.dart";
import "../../features/library/application/library_exact_analysis_scheduler.dart";
import "../../features/library/application/library_exact_duplicates.dart";
import "../../features/library/application/library_foreground_reads.dart";
import "../../features/library/application/library_gallery_duplicates.dart";
import "../../features/library/application/ports/library_exact_duplicates.dart";

class LibraryDuplicateBindings {
  LibraryDuplicateBindings.production()
    : exactDuplicates = const RustLibraryExactDuplicates();

  final LibraryExactDuplicates exactDuplicates;

  List<Override> get overrides => [
    libraryExactDuplicatesProvider.overrideWithValue(exactDuplicates),
    libraryGalleryDuplicatesProvider.overrideWithValue(
      const RustLibraryGalleryDuplicates(),
    ),
    libraryDuplicateReviewProvider.overrideWithValue(
      const RustLibraryDuplicateReview(),
    ),
  ];
}

class LibraryExactAnalysisBindings {
  LibraryExactAnalysisBindings({
    required LibraryExactDuplicates service,
    required bool paused,
    required LibraryForegroundReads foregroundReads,
  }) {
    analysis = LibraryExactAnalysisController(service);
    scheduler = LibraryExactAnalysisScheduler(
      analysis,
      paused: paused,
      foregroundReads: foregroundReads,
    );
  }

  late final LibraryExactAnalysisController analysis;
  late final LibraryExactAnalysisScheduler scheduler;
  Future<void>? _closing;

  List<Override> get overrides => [
    libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
    libraryExactAnalysisSchedulerProvider.overrideWithValue(scheduler),
  ];

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    try {
      await scheduler.close();
    } finally {
      scheduler.dispose();
      analysis.dispose();
    }
  }
}

final libraryDuplicateRequirements = <ProviderListenable<Object?>>[
  libraryExactDuplicatesProvider,
  libraryGalleryDuplicatesProvider,
  libraryDuplicateReviewProvider,
];

final libraryExactAnalysisRequirements = <ProviderListenable<Object?>>[
  libraryExactAnalysisControllerProvider,
  libraryExactAnalysisSchedulerProvider,
];
