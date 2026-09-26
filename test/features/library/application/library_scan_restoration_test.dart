import "dart:async";

import "package:cedarflake_ame/features/library/application/library_scan_restoration.dart";
import "package:cedarflake_ame/features/library/application/library_scan_session.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "interrupted first import restores progress without active execution",
    () async {
      final fixture = _Fixture(
        initial: const LibraryPrimaryScanSnapshot(
          status: LibraryStatus.completed,
        ),
        recoverable: () async => _checkpoint,
      );

      await fixture.restoration.restore();

      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.state.taskKind, LibraryTaskKind.import);
      expect(fixture.state.scanId, _checkpoint.scanId);
      expect(fixture.state.rootPath, _checkpoint.rootPath);
      expect(fixture.state.displayRootPath, _checkpoint.displayRootPath);
      expect(fixture.state.visitedEntries, 128);
      expect(fixture.state.stagedAssetCount, 40);
      expect(fixture.state.issueCount, 3);
      expect(fixture.state.itemLimit, 500);
      expect(fixture.state.entryLimit, 2000);
      expect(fixture.state.isResumingScan, isFalse);
      expect(fixture.session.activeScanId, isNull);
      expect(fixture.session.pausedScan, same(_checkpoint));
      expect(fixture.session.pausedScan?.previewEdge, 384);
      expect(fixture.pausedLoads, 0);

      await fixture.restoration.restore();
      expect(fixture.recoverableLoads, 1);
      expect(fixture.publications, 1);
    },
  );

  test("explicitly paused checkpoint also waits for manual continue", () async {
    final fixture = _Fixture(paused: () async => _checkpoint);

    await fixture.restoration.restore();

    expect(fixture.state.status, LibraryStatus.paused);
    expect(fixture.session.activeScanId, isNull);
    expect(fixture.session.pausedScan, same(_checkpoint));
    expect(fixture.recoverableLoads, 1);
    expect(fixture.pausedLoads, 1);
  });

  test("no eligible checkpoint leaves idle task state unchanged", () async {
    final initial = LibraryPrimaryScanSnapshot(status: LibraryStatus.completed);
    final fixture = _Fixture(initial: initial);

    await fixture.restoration.restore();

    expect(fixture.state, same(initial));
    expect(fixture.session.pausedScan, isNull);
    expect(fixture.publications, 0);
  });

  test("a cancelled task is never revived by a stale checkpoint", () async {
    const initial = LibraryPrimaryScanSnapshot(
      status: LibraryStatus.cancelled,
      scanId: "cancelled-import",
    );
    final fixture = _Fixture(
      initial: initial,
      recoverable: () async => _checkpoint,
    );

    await fixture.restoration.restore();

    expect(fixture.state, same(initial));
    expect(fixture.recoverableLoads, 0);
    expect(fixture.session.pausedScan, isNull);
  });

  test("late checkpoint cannot replace a newer completed operation", () async {
    final pending = Completer<RecoverableLibraryScan?>();
    final fixture = _Fixture(recoverable: () => pending.future);
    final restore = fixture.restoration.restore();
    fixture.sequence += 1;
    const current = LibraryPrimaryScanSnapshot(
      status: LibraryStatus.completed,
      scanId: "new-completed-import",
    );
    fixture.state = current;

    pending.complete(_checkpoint);
    await restore;

    expect(fixture.state, same(current));
    expect(fixture.pausedLoads, 0);
    expect(fixture.session.pausedScan, isNull);
  });

  test("late checkpoint cannot overwrite an active user scan", () async {
    final pending = Completer<RecoverableLibraryScan?>();
    final fixture = _Fixture(recoverable: () => pending.future);
    final restore = fixture.restoration.restore();
    const current = LibraryPrimaryScanSnapshot(
      status: LibraryStatus.scanning,
      scanId: "active-import",
    );
    fixture.state = current;

    pending.complete(_checkpoint);
    await restore;

    expect(fixture.state, same(current));
    expect(fixture.session.pausedScan, isNull);
  });

  for (final paused in [false, true]) {
    test(
      "shutdown or disposal suppresses late ${paused ? 'paused' : 'running'} checkpoint",
      () async {
        final pending = Completer<RecoverableLibraryScan?>();
        final fixture = _Fixture(
          recoverable: paused ? null : () => pending.future,
          paused: paused ? () => pending.future : null,
        );
        final restore = fixture.restoration.restore();
        await Future<void>.delayed(Duration.zero);
        fixture.unavailable = true;

        pending.complete(_checkpoint);
        await restore;

        expect(fixture.publications, 0);
        expect(fixture.session.pausedScan, isNull);
      },
    );
  }

  test("checkpoint failure is visible once without automatic retry", () async {
    final fixture = _Fixture(
      recoverable: () async => throw const LibraryScanFailure(
        code: "checkpoint_unavailable",
        message: "Cannot load the saved first import",
      ),
    );

    await fixture.restoration.restore();
    await fixture.restoration.restore();

    expect(fixture.state.status, LibraryStatus.failed);
    expect(fixture.state.errorMessage, contains("checkpoint_unavailable"));
    expect(fixture.recoverableLoads, 1);
    expect(fixture.pausedLoads, 0);
    expect(fixture.session.pausedScan, isNull);
  });

  test("late checkpoint failure cannot overwrite a newer task", () async {
    final pending = Completer<RecoverableLibraryScan?>();
    final fixture = _Fixture(recoverable: () => pending.future);
    final restore = fixture.restoration.restore();
    fixture.sequence += 1;
    const current = LibraryPrimaryScanSnapshot(status: LibraryStatus.completed);
    fixture.state = current;

    pending.completeError(StateError("stale read failure"));
    await restore;

    expect(fixture.state, same(current));
    expect(fixture.publications, 0);
  });
}

const _checkpoint = RecoverableLibraryScan(
  scanId: "interrupted-import",
  rootPath: r"\\?\C:\Pictures",
  displayRootPath: "C:\\Pictures",
  previewEdge: 384,
  itemLimit: 500,
  entryLimit: 2000,
  visitedEntries: 128,
  acceptedItems: 40,
  issueCount: 3,
);

class _Fixture {
  _Fixture({
    LibraryPrimaryScanSnapshot initial = const LibraryPrimaryScanSnapshot(),
    LibraryScanCheckpointLoader? recoverable,
    LibraryScanCheckpointLoader? paused,
  }) : state = initial {
    restoration = LibraryScanRestoration(
      loadRecoverable: () async {
        recoverableLoads += 1;
        return recoverable == null ? null : await recoverable();
      },
      loadPaused: () async {
        pausedLoads += 1;
        return paused == null ? null : await paused();
      },
      session: session,
      readState: () {
        expect(unavailable, isFalse);
        return state;
      },
      writeState: (next) {
        publications += 1;
        state = next;
      },
      readScanSequence: () => sequence,
      isUnavailable: () => unavailable,
    );
  }

  LibraryPrimaryScanSnapshot state;
  final session = LibraryScanSession();
  late final LibraryScanRestoration restoration;
  int sequence = 0;
  bool unavailable = false;
  int recoverableLoads = 0;
  int pausedLoads = 0;
  int publications = 0;
}
