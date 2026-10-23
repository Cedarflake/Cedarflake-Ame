import "dart:async";

import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_publication.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "earlier listeners see pending publication at stream retirement",
    () async {
      final service = _Service();
      final analysis = LibraryExactAnalysisController(service);
      addTearDown(analysis.dispose);
      late LibraryExactAnalysisPublication publication;
      final observed = <bool>[];
      analysis.addListener(() {
        if (!analysis.isRunning) {
          observed.add(publication.isPublishing);
        }
      });
      final completion = Completer<bool>();
      publication = LibraryExactAnalysisPublication(
        analysis,
        refresh: () => completion.future,
      );
      addTearDown(publication.dispose);
      analysis.start();
      service.emit(LibraryExactAnalysisPhase.completed);
      await service.finish();
      expect(observed, [true]);
      completion.complete(true);
      await Future<void>.delayed(Duration.zero);
      expect(publication.isPublishing, isFalse);
    },
  );

  test(
    "completion waits for gallery publication and retains retryable failure",
    () async {
      final service = _Service();
      final analysis = LibraryExactAnalysisController(service);
      addTearDown(analysis.dispose);
      final first = Completer<bool>();
      var attempts = 0;
      final publication = LibraryExactAnalysisPublication(
        analysis,
        refresh: () {
          attempts++;
          return attempts == 1 ? first.future : Future.value(true);
        },
      );
      addTearDown(publication.dispose);
      analysis.start();
      service.emit(LibraryExactAnalysisPhase.completed);
      await service.finish();
      await analysis.whenStopped;
      expect(publication.isPublishing, isTrue);
      first.complete(false);
      await Future<void>.delayed(Duration.zero);
      expect(publication.isPublishing, isFalse);
      expect(publication.hasFailure, isTrue);
      publication.retry();
      await Future<void>.delayed(Duration.zero);
      expect(publication.hasFailure, isFalse);
      expect(attempts, 2);
    },
  );
  for (final phase in [
    LibraryExactAnalysisPhase.completed,
    LibraryExactAnalysisPhase.cancelled,
    LibraryExactAnalysisPhase.failed,
  ]) {
    test(
      "refreshes retained evidence once after stream retirement: $phase",
      () async {
        final service = _Service();
        final analysis = LibraryExactAnalysisController(service);
        addTearDown(analysis.dispose);
        var refreshes = 0;
        final publication = LibraryExactAnalysisPublication(
          analysis,
          refresh: () async {
            refreshes += 1;
            return true;
          },
        );
        addTearDown(publication.dispose);
        analysis.start();
        service.emit(phase);
        await Future<void>.delayed(Duration.zero);
        expect(refreshes, 0);
        await service.finish();
        await analysis.whenStopped;
        expect(refreshes, 1);
        await analysis.stop();
        expect(refreshes, 1);
      },
    );
  }

  test(
    "later completions coalesce behind the current gallery publication",
    () async {
      final service = _Service();
      final analysis = LibraryExactAnalysisController(service);
      addTearDown(analysis.dispose);
      final first = Completer<bool>();
      var refreshes = 0;
      final publication = LibraryExactAnalysisPublication(
        analysis,
        refresh: () {
          refreshes += 1;
          return refreshes == 1 ? first.future : Future.value(true);
        },
      );
      addTearDown(publication.dispose);
      for (var run = 0; run < 3; run += 1) {
        analysis.start();
        service.emit(LibraryExactAnalysisPhase.completed);
        await service.finish();
        await analysis.whenStopped;
      }
      expect(refreshes, 1);
      first.complete(false);
      await Future<void>.delayed(Duration.zero);
      expect(refreshes, 2);
    },
  );

  test("disposing retires queued and later analysis publications", () async {
    final service = _Service();
    final analysis = LibraryExactAnalysisController(service);
    addTearDown(analysis.dispose);
    final first = Completer<bool>();
    var refreshes = 0;
    final publication = LibraryExactAnalysisPublication(
      analysis,
      refresh: () {
        refreshes += 1;
        return first.future;
      },
    );
    for (var run = 0; run < 2; run += 1) {
      analysis.start();
      service.emit(LibraryExactAnalysisPhase.completed);
      await service.finish();
      await analysis.whenStopped;
    }
    publication.dispose();
    first.complete(true);
    await Future<void>.delayed(Duration.zero);
    analysis.start();
    service.emit(LibraryExactAnalysisPhase.completed);
    await service.finish();
    expect(refreshes, 1);
  });
}

class _Service implements LibraryExactDuplicates {
  late StreamController<LibraryExactAnalysisProgress> _stream;
  late String _runId;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    final stream = StreamController<LibraryExactAnalysisProgress>.broadcast();
    stream.onCancel = () => unawaited(stream.close());
    _stream = stream;
    _runId = runId;
    return stream.stream;
  }

  void emit(LibraryExactAnalysisPhase phase) => _stream.add(
    LibraryExactAnalysisProgress(
      runId: _runId,
      phase: phase,
      checkedFiles: 0,
      fingerprintedFiles: 0,
      skippedFiles: 0,
    ),
  );

  Future<void> finish() => _stream.close();

  @override
  Future<void> cancel(String runId) async {}

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
