import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "closing during startup retires the late handle without running",
    (tester) async {
      final fixture = _Fixture();
      fixture.owner.startInBackground();
      final closed = fixture.owner.close();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      await closed;
      expect(fixture.session.runs, 0);
      expect(fixture.session.closed, isTrue);
      fixture.dispose();
    },
  );

  testWidgets("unconfigured startup remains inactive and closes normally", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    fixture.owner.startInBackground();
    fixture.service.opened.complete(null);
    await tester.pump();
    await fixture.owner.close();
    expect(fixture.service.opens, 1);
    expect(fixture.session.runs, 0);
    fixture.dispose();
  });

  testWidgets(
    "late startup receives current reads, import, and suspend before running",
    (tester) async {
      final fixture = _Fixture();
      fixture.owner.startInBackground();
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.update(2, busy: true, roots: {"root": BigInt.two});
      final read = fixture.reads.acquire();
      fixture.owner.setSuspended(true);
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      expect(fixture.session.activityAtRun, (true, true, true, false));
      expect(fixture.session.recovered, [("root", BigInt.two)]);
      read.close();
      fixture.owner.setSuspended(false);
      fixture.update(3);
      await tester.pump();
      expect(fixture.session.activity, (false, false, false, false));
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("close waits for the running task and in-flight recovery", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.session.autoFinish = false;
    fixture.session.recoveryGate = Completer<void>();
    fixture.update(1, roots: {"root": BigInt.one});
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    var completed = false;
    final closed = fixture.owner.close().then((_) => completed = true);
    await tester.pump();
    expect(fixture.session.closeRequested, isTrue);
    expect(completed, isFalse);
    fixture.session.finished.complete();
    await tester.pump();
    expect(completed, isFalse);
    fixture.session.recoveryGate!.complete();
    await tester.pump();
    await closed;
    expect(fixture.session.closed, isTrue);
    fixture.dispose();
  });

  testWidgets(
    "catalog updates coalesce and returned roots recover without restarting work",
    (tester) async {
      final fixture = _Fixture();
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      final notifications = fixture.session.catalogNotifications;
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.owner.browsingActivity();
      expect(fixture.session.catalogNotifications, notifications);
      fixture.update(2);
      fixture.update(3, roots: {"root": BigInt.one});
      await tester.pump();
      expect(fixture.session.recovered, [
        ("root", BigInt.one),
        ("root", BigInt.one),
      ]);
      expect(fixture.session.runs, 1);
      expect(fixture.session.closeRequested, isFalse);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("pressure releases resources and retains a bounded cooldown", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.update(1);
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    fixture.owner.memoryPressure();
    expect(fixture.session.activity.$4, isTrue);
    await tester.pump(const Duration(seconds: 59));
    expect(fixture.session.activity.$4, isTrue);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.session.activity.$4, isFalse);
    expect(fixture.session.runs, 1);
    await fixture.owner.close();
    fixture.dispose();
  });
}

class _Fixture {
  final service = _Service();
  final session = _Session();
  final reads = LibraryForegroundReads();
  late final owner = LibraryClassificationLifecycle(
    service,
    foregroundReads: reads,
  );

  void update(
    int revision, {
    bool busy = false,
    Map<String, BigInt> roots = const {},
  }) => owner.updateCatalog(
    revision: BigInt.from(revision),
    availableRoots: roots,
    isBusy: busy,
  );

  void dispose() {
    owner.dispose();
    reads.dispose();
  }
}

class _Service implements LibraryClassification {
  final opened = Completer<LibraryClassificationSession?>();
  int opens = 0;
  @override
  Future<LibraryClassificationSession?> open() {
    opens++;
    return opened.future;
  }
}

class _Session implements LibraryClassificationSession {
  final finished = Completer<void>();
  Completer<void>? recoveryGate;
  bool autoFinish = true;
  bool closeRequested = false;
  bool closed = false;
  int runs = 0;
  int catalogNotifications = 0;
  final recovered = <(String, BigInt)>[];
  (bool, bool, bool, bool) activity = (false, false, true, false);
  (bool, bool, bool, bool)? activityAtRun;

  @override
  Future<void> run() {
    runs++;
    activityAtRun = activity;
    return finished.future;
  }

  @override
  void foregroundChanged({
    required bool readingSources,
    required bool importing,
    required bool suspended,
    required bool memoryPressure,
  }) {
    activity = (readingSources, importing, suspended, memoryPressure);
  }

  @override
  void browsingActivity() {}
  @override
  void catalogChanged() => catalogNotifications++;
  @override
  Future<void> rootAvailable(String rootId, BigInt generation) async {
    recovered.add((rootId, generation));
    await recoveryGate?.future;
  }

  @override
  void requestClose() {
    closeRequested = true;
    if (autoFinish && !finished.isCompleted) {
      finished.complete();
    }
  }

  @override
  Future<void> close() async {
    closed = true;
  }
}
