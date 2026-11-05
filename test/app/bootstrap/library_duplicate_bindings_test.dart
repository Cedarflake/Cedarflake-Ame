import "dart:async";

import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_duplicate_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_similar_review.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_similar_review_controller.dart";
import "package:cedarflake_ame/features/library/application/ports/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final requirements = [
    ...libraryDuplicateRequirements,
    ...libraryExactAnalysisRequirements,
  ];

  test(
    "production ports and analysis share one lifecycle and foreground owner",
    () async {
      final foreground = _Foreground();
      final ports = LibraryDuplicateBindings.production();
      final runtime = LibraryExactAnalysisBindings(
        service: ports.exactDuplicates,
        paused: true,
        foregroundReads: foreground,
      );
      final shutdown = AmeShutdownCoordinator();
      final assembly = AmeProviderAssembly(shutdown: shutdown);
      shutdown.register(runtime.close);
      final container = assembly.assemble(
        requirements: requirements,
        overrides: [...ports.overrides, ...runtime.overrides],
      );
      final child = ProviderContainer(parent: container);
      expect(
        container.read(libraryExactDuplicatesProvider),
        same(ports.exactDuplicates),
      );
      expect(ports.exactDuplicates, isA<RustLibraryExactDuplicates>());
      expect(
        container.read(libraryGalleryDuplicatesProvider),
        isA<RustLibraryGalleryDuplicates>(),
      );
      expect(
        container.read(libraryDuplicateReviewProvider),
        isA<RustLibraryDuplicateReview>(),
      );
      expect(
        container.read(librarySimilarReviewProvider),
        isA<RustLibrarySimilarReview>(),
      );
      expect(
        child.read(libraryExactAnalysisControllerProvider),
        same(runtime.analysis),
      );
      expect(
        child.read(libraryExactAnalysisSchedulerProvider),
        same(runtime.scheduler),
      );
      expect(runtime.scheduler.analysis, same(runtime.analysis));
      expect(runtime.scheduler.foregroundReads, same(foreground));
      expect(runtime.scheduler.isPaused, isTrue);
      child.dispose();
      await shutdown.shutdown();
      await runtime.close();
      expect(
        () => ChangeNotifier.debugAssertNotDisposed(runtime.analysis),
        throwsFlutterError,
      );
      expect(foreground.isObserved, isFalse);
      foreground.dispose();
    },
  );

  test(
    "every missing duplicate port or runtime binding fails during assembly",
    () async {
      final foreground = _Foreground();
      final ports = LibraryDuplicateBindings.production();
      final runtime = LibraryExactAnalysisBindings(
        service: ports.exactDuplicates,
        paused: false,
        foregroundReads: foreground,
      );
      final overrides = [...ports.overrides, ...runtime.overrides];
      for (final omitted in requirements) {
        expect(
          () =>
              AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
                requirements: requirements,
                overrides: overrides
                    .where((value) => !identical(value.origin, omitted))
                    .toList(),
              ),
          throwsA(isA<Exception>()),
        );
      }
      await runtime.close();
      foreground.dispose();
    },
  );

  test(
    "close joins native completion before retiring analysis and subscriptions",
    () async {
      final service = _Analysis();
      final foreground = _Foreground();
      final runtime = LibraryExactAnalysisBindings(
        service: service,
        paused: false,
        foregroundReads: foreground,
      );
      runtime.analysis.start();
      await Future<void>.delayed(Duration.zero);
      final closing = runtime.close();
      expect(runtime.close(), same(closing));
      var closed = false;
      unawaited(closing.then((_) => closed = true));
      await Future<void>.delayed(Duration.zero);
      expect(service.cancellations, [service.runId]);
      expect(closed, isFalse);
      expect(runtime.analysis.isRunning, isTrue);
      service.finish();
      await closing;
      expect(runtime.analysis.isRunning, isFalse);
      expect(service.events.hasListener, isFalse);
      expect(foreground.isObserved, isFalse);
      foreground.dispose();
    },
  );

  test("later startup failure closes the acquired analysis runtime", () async {
    final foreground = _Foreground();
    final runtime = LibraryExactAnalysisBindings(
      service: _Analysis(),
      paused: false,
      foregroundReads: foreground,
    );
    final shutdown = AmeShutdownCoordinator();
    final assembly = AmeProviderAssembly(shutdown: shutdown);
    shutdown.register(runtime.close);
    await expectLater(
      acquireAmeApplication(
        shutdown: shutdown,
        acquire: () async => assembly.assemble(
          overrides: runtime.overrides,
          requirements: requirements,
        ),
      ),
      throwsA(isA<Exception>()),
    );
    expect(foreground.isObserved, isFalse);
    expect(
      () => ChangeNotifier.debugAssertNotDisposed(runtime.analysis),
      throwsFlutterError,
    );
    foreground.dispose();
  });
}

class _Foreground extends LibraryForegroundReads {
  bool get isObserved => hasListeners;
}

class _Analysis implements LibraryExactDuplicates {
  final events = StreamController<LibraryExactAnalysisProgress>();
  final cancellations = <String>[];
  String? runId;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    this.runId = runId;
    events.add(
      LibraryExactAnalysisProgress(
        runId: runId,
        phase: LibraryExactAnalysisPhase.running,
        checkedFiles: 0,
        fingerprintedFiles: 0,
        skippedFiles: 0,
      ),
    );
    return events.stream;
  }

  @override
  Future<void> cancel(String runId) async {
    cancellations.add(runId);
  }

  void finish() {
    events.add(
      LibraryExactAnalysisProgress(
        runId: runId!,
        phase: LibraryExactAnalysisPhase.cancelled,
        checkedFiles: 1,
        fingerprintedFiles: 0,
        skippedFiles: 1,
      ),
    );
    unawaited(events.close());
  }

  @override
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  ) async => const LibraryDuplicateGroupPage(groups: []);

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async => const LibraryDuplicateLocationPage(locations: []);
}
