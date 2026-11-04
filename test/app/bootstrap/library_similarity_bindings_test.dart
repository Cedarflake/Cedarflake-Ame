import "dart:async";

import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_similarity_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_similar_analysis.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_similar_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_similar_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/ports/library_similar_analysis.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_analysis.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "production similarity bindings are required and remain cold until admission",
    () async {
      final foreground = _Foreground();
      final runtime = LibrarySimilarAnalysisBindings.production(
        foregroundReads: foreground,
      );
      final shutdown = AmeShutdownCoordinator();
      final assembly = AmeProviderAssembly(shutdown: shutdown);
      shutdown.register(runtime.close);
      final container = assembly.assemble(
        overrides: runtime.overrides,
        requirements: librarySimilarAnalysisRequirements,
      );
      expect(
        container.read(librarySimilarAnalysisProvider),
        isA<RustLibrarySimilarAnalysis>(),
      );
      expect(
        container.read(librarySimilarAnalysisControllerProvider),
        same(runtime.analysis),
      );
      expect(
        container.read(librarySimilarAnalysisSchedulerProvider),
        same(runtime.scheduler),
      );
      expect(runtime.analysis.isRunning, isFalse);
      for (final omitted in librarySimilarAnalysisRequirements) {
        expect(
          () =>
              AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
                requirements: librarySimilarAnalysisRequirements,
                overrides: runtime.overrides
                    .where((value) => !identical(value.origin, omitted))
                    .toList(),
              ),
          throwsA(isA<Exception>()),
        );
      }
      await shutdown.shutdown();
      expect(foreground.isObserved, isFalse);
      foreground.dispose();
    },
  );

  testWidgets(
    "close before native open joins cancellation before handle disposal",
    (tester) async {
      final service = _Service(deferredOpen: true);
      final foreground = _Foreground();
      final runtime = LibrarySimilarAnalysisBindings(
        service: service,
        foregroundReads: foreground,
      );
      runtime.analysis.start();
      final closing = runtime.close();
      expect(runtime.close(), same(closing));
      var closed = false;
      unawaited(closing.then((_) => closed = true));
      await tester.pump();
      final session = service.sessions.single;
      expect(session.events, isEmpty);
      expect(closed, isFalse);
      service.opening.complete(session);
      await tester.pump();
      expect(session.events, ["cancel", "suspend:false", "run"]);
      expect(session.disposed, isFalse);
      expect(runtime.analysis.isRunning, isTrue);
      session.finish(LibrarySimilarAnalysisPhase.cancelled);
      await tester.pump();
      await closing;
      expect(runtime.analysis.error, isNull);
      expect(session.events.last, "dispose");
      expect(closed, isTrue);
      expect(foreground.isObserved, isFalse);
      foreground.dispose();
    },
  );

  testWidgets(
    "automatic analysis coalesces publications and preserves foreground and retry admission",
    (tester) async {
      final service = _Service();
      final foreground = _Foreground();
      final runtime = LibrarySimilarAnalysisBindings(
        service: service,
        foregroundReads: foreground,
      );
      final scheduler = runtime.scheduler;
      void update(int revision) => scheduler.updateCatalog(
        revision: BigInt.from(revision),
        roots: const {"root": LibraryRootAvailability.available},
        isBusy: false,
      );
      scheduler.setSuspended(false);
      update(1);
      await tester.pump(const Duration(seconds: 2));
      expect(service.sessions, isEmpty);
      final openingRead = foreground.acquire();
      await tester.pump();
      await tester.pump(const Duration(seconds: 3));
      expect(service.sessions, isEmpty);
      openingRead.close();
      await tester.pump();
      await tester.pump(const Duration(seconds: 3));
      expect(service.sessions, hasLength(1));
      final first = service.sessions.single;
      scheduler.browsingActivity();
      update(2);
      await tester.pump();
      expect(first.cancelled, isFalse);
      expect(service.sessions, hasLength(1));
      final viewing = foreground.acquire();
      await tester.pump();
      expect(first.suspended, isTrue);
      expect(first.cancelled, isFalse);
      viewing.close();
      await tester.pump();
      expect(first.suspended, isFalse);
      first.finish(LibrarySimilarAnalysisPhase.completed);
      await tester.pump();
      await tester.pump(const Duration(seconds: 3));
      expect(service.sessions, hasLength(2));
      service.sessions.last.finish(LibrarySimilarAnalysisPhase.failed);
      await tester.pump();
      scheduler.browsingActivity();
      await tester.pump(const Duration(seconds: 30));
      scheduler.browsingActivity();
      expect(service.sessions, hasLength(2));
      await tester.pump(const Duration(seconds: 30));
      await tester.pump();
      expect(service.sessions, hasLength(3));
      scheduler.memoryPressure();
      await tester.pump();
      expect(service.sessions.last.cancelled, isTrue);
      service.sessions.last.finish(LibrarySimilarAnalysisPhase.cancelled);
      await tester.pump();
      await tester.pump(const Duration(seconds: 3));
      expect(service.sessions, hasLength(3));
      await tester.pump(const Duration(seconds: 57));
      await tester.pump();
      expect(service.sessions, hasLength(4));
      service.sessions.last.finish(LibrarySimilarAnalysisPhase.completed);
      await tester.pump();
      expect(runtime.analysis.error, isNull);
      update(2);
      await tester.pump(const Duration(minutes: 1));
      expect(service.sessions, hasLength(4));
      expect(scheduler.isWaiting, isFalse);
      await runtime.close();
      expect(service.sessions.every((session) => session.disposed), isTrue);
      foreground.dispose();
    },
  );
}

class _Foreground extends LibraryForegroundReads {
  bool get isObserved => hasListeners;
}

class _Service implements LibrarySimilarAnalysis {
  _Service({this.deferredOpen = false});
  final bool deferredOpen;
  final opening = Completer<LibrarySimilarAnalysisSession>();
  final sessions = <_Session>[];

  @override
  Future<LibrarySimilarAnalysisSession> open(String runId) {
    final session = _Session(runId);
    sessions.add(session);
    return deferredOpen ? opening.future : Future.value(session);
  }
}

class _Session implements LibrarySimilarAnalysisSession {
  _Session(this.runId);
  final String runId;
  final completion = Completer<LibrarySimilarAnalysisProgress>();
  final events = <String>[];
  bool cancelled = false;
  bool suspended = false;
  bool disposed = false;

  void finish(LibrarySimilarAnalysisPhase phase) => completion.complete(
    LibrarySimilarAnalysisProgress(runId: runId, phase: phase),
  );

  @override
  Future<LibrarySimilarAnalysisProgress> run() {
    events.add("run");
    return completion.future;
  }

  @override
  LibrarySimilarAnalysisProgress progress() => LibrarySimilarAnalysisProgress(
    runId: runId,
    phase: LibrarySimilarAnalysisPhase.running,
  );

  @override
  void cancel() {
    events.add("cancel");
    cancelled = true;
  }

  @override
  void suspend(bool value) {
    events.add("suspend:$value");
    suspended = value;
  }

  @override
  void dispose() {
    expectSync(completion.isCompleted, isTrue);
    events.add("dispose");
    disposed = true;
  }
}
