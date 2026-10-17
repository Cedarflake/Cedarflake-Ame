import "dart:async";

import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("idle publication starts once and unchanged input stays idle", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1);
    await tester.pump(const Duration(seconds: 2));
    expect(fixture.service.runs, isEmpty);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.service.runs, hasLength(1));
    fixture.service.finish(LibraryExactAnalysisPhase.completed);
    await tester.pump();
    fixture.update(1);
    await tester.pump(const Duration(minutes: 1));
    expect(fixture.service.runs, hasLength(1));
    expect(fixture.scheduler.isWaiting, isFalse);
  });

  testWidgets("synchronization and later input restart the idle interval", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1, busy: true);
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, isEmpty);
    fixture.update(1);
    await tester.pump(const Duration(seconds: 2));
    fixture.scheduler.browsingActivity();
    await tester.pump(const Duration(seconds: 2));
    expect(fixture.service.runs, isEmpty);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.service.runs, hasLength(1));
  });

  testWidgets("browsing cancels active work and resumes after native drain", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.service.completeCancellation = false;
    fixture.update(1);
    await tester.pump(const Duration(seconds: 3));
    fixture.scheduler.browsingActivity();
    await tester.pump(const Duration(seconds: 5));
    expect(fixture.service.cancellations, hasLength(1));
    expect(fixture.service.runs, hasLength(1));
    fixture.service.finish(LibraryExactAnalysisPhase.cancelled);
    await tester.pump();
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(2));
  });

  testWidgets("a newer publication survives completion of an older run", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1);
    await tester.pump(const Duration(seconds: 3));
    fixture.update(2);
    fixture.service.finish(LibraryExactAnalysisPhase.completed);
    await tester.pump();
    expect(fixture.scheduler.isWaiting, isTrue);
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(2));
    fixture.service.finish(LibraryExactAnalysisPhase.completed);
    await tester.pump();
    expect(fixture.scheduler.isWaiting, isFalse);
  });

  testWidgets("failure waits for an explicit retry or new publication", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1);
    await tester.pump(const Duration(seconds: 3));
    fixture.service.finish(LibraryExactAnalysisPhase.failed);
    await tester.pump();
    fixture.scheduler.browsingActivity();
    await tester.pump(const Duration(minutes: 1));
    expect(fixture.service.runs, hasLength(1));
    fixture.scheduler.requestNow();
    await tester.pump(Duration.zero);
    expect(fixture.service.runs, hasLength(2));
    fixture.service.finish(LibraryExactAnalysisPhase.failed);
    await tester.pump();
    fixture.update(2);
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(3));
  });

  testWidgets("saved pause blocks startup and resume processes pending work", (
    tester,
  ) async {
    final fixture = _Fixture(paused: true);
    addTearDown(fixture.dispose);
    fixture.update(1);
    await tester.pump(const Duration(minutes: 1));
    expect(fixture.service.runs, isEmpty);
    fixture.scheduler.setPaused(false);
    await tester.pump(const Duration(seconds: 3));
    fixture.scheduler.setPaused(true);
    await tester.pump();
    expect(fixture.service.cancellations, hasLength(1));
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, hasLength(1));
    fixture.scheduler.setPaused(false);
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(2));
  });

  testWidgets("foreground reads retain priority until the last lease closes", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1);
    final preview = fixture.foreground.acquire();
    final original = fixture.foreground.acquire();
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, isEmpty);
    preview.close();
    preview.close();
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, isEmpty);
    original.close();
    await tester.pump();
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(1));
    final nextImage = fixture.foreground.acquire();
    await tester.pump();
    expect(fixture.service.cancellations, hasLength(1));
    nextImage.close();
    await tester.pump();
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(2));
  });

  testWidgets("root changes schedule work without a new catalog revision", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.update(1, roots: const {});
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, isEmpty);
    fixture.update(1);
    await tester.pump(const Duration(seconds: 3));
    fixture.service.finish(LibraryExactAnalysisPhase.completed);
    await tester.pump();
    fixture.update(
      1,
      roots: const {"second": LibraryRootAvailability.available},
    );
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.service.runs, hasLength(2));
  });

  testWidgets("shutdown fences late admission and shares its drain future", (
    tester,
  ) async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.service.admitImmediately = false;
    fixture.update(1);
    await tester.pump(const Duration(seconds: 3));
    var closed = false;
    final closing = fixture.scheduler.close();
    expect(identical(closing, fixture.scheduler.close()), isTrue);
    unawaited(closing.then((_) => closed = true));
    await tester.pump();
    expect(closed, isFalse);
    expect(fixture.service.cancellations, isEmpty);
    fixture.service.emit(LibraryExactAnalysisPhase.running);
    await tester.pump();
    expect(closed, isTrue);
    expect(fixture.service.cancellations, hasLength(1));
    fixture.update(2);
    await tester.pump(const Duration(seconds: 10));
    expect(fixture.service.runs, hasLength(1));
  });
}

class _Fixture {
  _Fixture({bool paused = false}) {
    analysis = LibraryExactAnalysisController(service);
    scheduler = LibraryExactAnalysisScheduler(
      analysis,
      paused: paused,
      foregroundReads: foreground,
    );
  }
  final service = _Service();
  final foreground = LibraryForegroundReads();
  late final LibraryExactAnalysisController analysis;
  late final LibraryExactAnalysisScheduler scheduler;

  void update(
    int revision, {
    bool busy = false,
    Map<String, LibraryRootAvailability> roots = const {
      "root": LibraryRootAvailability.available,
    },
  }) => scheduler.updateCatalog(
    revision: BigInt.from(revision),
    roots: roots,
    isBusy: busy,
  );

  void dispose() {
    scheduler.dispose();
    analysis.dispose();
    foreground.dispose();
    for (final stream in service.runs.values) {
      unawaited(stream.close());
    }
  }
}

class _Service implements LibraryExactDuplicates {
  final runs = <String, StreamController<LibraryExactAnalysisProgress>>{};
  final cancellations = <String>[];
  bool admitImmediately = true;
  bool completeCancellation = true;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    final stream = StreamController<LibraryExactAnalysisProgress>.broadcast();
    stream.onCancel = () => unawaited(stream.close());
    runs[runId] = stream;
    if (admitImmediately) {
      scheduleMicrotask(() => emit(LibraryExactAnalysisPhase.running));
    }
    return stream.stream;
  }

  void emit(LibraryExactAnalysisPhase phase) {
    final run = runs.entries.last;
    if (!run.value.isClosed) {
      run.value.add(
        LibraryExactAnalysisProgress(
          runId: run.key,
          phase: phase,
          checkedFiles: 1,
          fingerprintedFiles: 1,
          skippedFiles: 0,
        ),
      );
    }
  }

  void finish(LibraryExactAnalysisPhase phase) {
    emit(phase);
    unawaited(runs.values.last.close());
  }

  @override
  Future<void> cancel(String runId) async {
    cancellations.add(runId);
    if (completeCancellation) {
      finish(LibraryExactAnalysisPhase.cancelled);
    }
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
