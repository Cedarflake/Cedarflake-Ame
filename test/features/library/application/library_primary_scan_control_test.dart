import "package:cedarflake_ame/features/library/application/library_scan_control.dart";
import "package:cedarflake_ame/features/library/application/library_scan_shutdown.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/primary_scan_control_fixture.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  for (final command in [LibraryScanCommand.cancel, LibraryScanCommand.pause]) {
    for (final registered in [false, true]) {
      test(
        "${command.name} is retained before Started with native registered=$registered",
        () async {
          final fixture = PrimaryScanControlFixture();
          final id = await fixture.start();
          if (registered) {
            fixture.scanner.registered.add(id);
          }
          await fixture.request(command);
          expect(
            fixture.state.status,
            command == LibraryScanCommand.cancel
                ? LibraryStatus.cancelling
                : LibraryStatus.pausing,
          );
          fixture.scanner.started(id);
          fixture.scanner.started(id);
          expect(
            fixture.scanner.commands,
            List.filled(registered ? 1 : 2, command),
          );
          expect(
            fixture.state.status,
            command == LibraryScanCommand.cancel
                ? LibraryStatus.cancelling
                : LibraryStatus.pausing,
          );
          fixture.scanner.emit(
            id,
            command == LibraryScanCommand.cancel
                ? const LibraryScanCancelled(acceptedItems: 0, issueCount: 0)
                : const LibraryScanPaused(
                    visitedEntries: 0,
                    acceptedItems: 0,
                    issueCount: 0,
                  ),
          );
          await fixture.scanner.close(id);
          expect(
            fixture.state.status,
            command == LibraryScanCommand.cancel
                ? LibraryStatus.cancelled
                : LibraryStatus.paused,
          );
        },
      );
    }
  }

  test(
    "cancel replaces pending pause and Started cannot downgrade its feedback",
    () async {
      final fixture = PrimaryScanControlFixture();
      final id = await fixture.start();
      fixture.controller.pauseScan();
      await fixture.controller.cancelScan();
      fixture.controller.pauseScan();
      fixture.scanner.started(id);
      fixture.scanner.emit(
        id,
        const LibraryScanProgress(
          visitedEntries: 8,
          acceptedItems: 3,
          issueCount: 0,
        ),
      );
      expect(fixture.scanner.commands, [
        LibraryScanCommand.pause,
        LibraryScanCommand.cancel,
        LibraryScanCommand.cancel,
      ]);
      expect(fixture.state.status, LibraryStatus.cancelling);
      expect(fixture.state.visitedEntries, 8);
    },
  );

  for (final terminal in <LibraryScanUpdate>[
    const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
    const LibraryScanStale(acceptedItems: 0, issueCount: 0),
    const LibraryScanFailed(code: "source_unavailable", message: "unavailable"),
  ]) {
    test(
      "${terminal.runtimeType} before Started retires pending controls",
      () async {
        final fixture = PrimaryScanControlFixture();
        final id = await fixture.start();
        await fixture.controller.cancelScan();
        fixture.scanner.emit(id, terminal);
        final terminalState = fixture.state.primaryScan;
        fixture.scanner.started(id);
        await fixture.controller.cancelScan();
        fixture.controller.pauseScan();
        expect(fixture.scanner.commands, [LibraryScanCommand.cancel]);
        expect(fixture.state.primaryScan, same(terminalState));
      },
    );
  }

  test(
    "wrong-ID Started fails its run and cannot be revived by a later correct event",
    () async {
      final fixture = PrimaryScanControlFixture();
      final id = await fixture.start();
      fixture.scanner.emit(
        id,
        const LibraryScanStarted(scanId: "foreign", rootPath: "foreign"),
      );
      expect(fixture.state.status, LibraryStatus.cancelling);
      expect(fixture.state.errorMessage, contains("bridge_scan_id_mismatch"));
      fixture.scanner.started(id);
      expect(fixture.state.status, LibraryStatus.cancelling);
      expect(fixture.scanner.controlIds, [id, id]);
      await fixture.scanner.close(id);
      expect(fixture.state.status, LibraryStatus.failed);
      expect(fixture.state.errorMessage, contains("bridge_scan_id_mismatch"));
      final nextId = await fixture.start();
      expect(nextId, isNot(id));
      fixture.scanner.started(nextId);
      expect(fixture.state.scanId, nextId);
      expect(fixture.state.status, LibraryStatus.scanning);
    },
  );

  for (final streamError in [false, true]) {
    test(
      "${streamError ? 'stream error' : 'wrong-ID Started'} retains admission and shutdown until native drain",
      () async {
        final fixture = PrimaryScanControlFixture();
        final id = await fixture.start();
        if (streamError) {
          fixture.scanner.fail(id);
        } else {
          fixture.scanner.emit(
            id,
            const LibraryScanStarted(scanId: "foreign", rootPath: "foreign"),
          );
        }
        expect(fixture.state.status, LibraryStatus.cancelling);
        var nextFinished = false;
        final next = fixture.controller
            .scanDirectory("C:\\Next")
            .then((_) => nextFinished = true);
        await flushRetainedScanMicrotasks();
        expect(nextFinished, isFalse);
        expect(fixture.state.scanId, id);
        var shutdownFinished = false;
        final shutdown = fixture.container
            .read(libraryScanShutdownCoordinatorProvider)
            .suspend()
            .then((_) => shutdownFinished = true);
        fixture.scanner.started(id);
        fixture.scanner.emit(
          id,
          const LibraryScanProgress(
            visitedEntries: 999,
            acceptedItems: 9,
            issueCount: 0,
          ),
        );
        await flushRetainedScanMicrotasks();
        expect(shutdownFinished, isFalse);
        expect(fixture.state.status, LibraryStatus.cancelling);
        expect(fixture.state.visitedEntries, isNot(999));
        expect(fixture.scanner.commands, [
          LibraryScanCommand.cancel,
          LibraryScanCommand.cancel,
        ]);
        expect(fixture.scanner.controlIds, [id, id]);
        await fixture.scanner.close(id);
        await next;
        await shutdown;
        expect(shutdownFinished, isTrue);
        expect(fixture.state.scanId, id);
        expect(fixture.state.status, LibraryStatus.failed);
      },
    );
  }

  test("shutdown fences a command still awaiting primary admission", () async {
    final fixture = PrimaryScanControlFixture();
    fixture.controller;
    await flushRetainedScanMicrotasks();
    final start = fixture.controller.scanDirectory(retainedScanRoot.path);
    final shutdown = fixture.container
        .read(libraryScanShutdownCoordinatorProvider)
        .suspend();
    await start;
    await shutdown;
    expect(fixture.state.scanId, isNull);
    expect(fixture.scanner.commands, isEmpty);
  });

  test(
    "control bridge failure is observable and pending intent survives registration",
    () async {
      final fixture = PrimaryScanControlFixture();
      final id = await fixture.start();
      fixture.scanner.failNextControl = true;
      await fixture.controller.cancelScan();
      expect(fixture.state.status, LibraryStatus.cancelling);
      expect(
        fixture.state.errorMessage,
        contains("bridge_scan_control_failed"),
      );
      fixture.scanner.started(id);
      expect(fixture.scanner.commands, [
        LibraryScanCommand.cancel,
        LibraryScanCommand.cancel,
      ]);
      expect(fixture.state.errorMessage, isNull);
    },
  );

  for (final wasCancelled in [false, true]) {
    test(
      "shutdown before registration drains ${wasCancelled ? 'cancel' : 'suspend'} ownership",
      () async {
        final fixture = PrimaryScanControlFixture();
        final id = await fixture.start();
        if (wasCancelled) {
          await fixture.controller.cancelScan();
        }
        var completed = false;
        final shutdown = fixture.container
            .read(libraryScanShutdownCoordinatorProvider)
            .suspend()
            .then((_) => completed = true);
        await flushRetainedScanMicrotasks();
        expect(completed, isFalse);
        final command = wasCancelled
            ? LibraryScanCommand.cancel
            : LibraryScanCommand.suspend;
        expect(fixture.scanner.commands, [command]);
        fixture.scanner.started(id);
        expect(fixture.scanner.commands, [command, command]);
        fixture.scanner.emit(
          id,
          wasCancelled
              ? const LibraryScanCancelled(acceptedItems: 0, issueCount: 0)
              : const LibraryScanPaused(
                  visitedEntries: 2,
                  acceptedItems: 1,
                  issueCount: 0,
                ),
        );
        expect(completed, isFalse);
        await fixture.scanner.close(id);
        await shutdown;
        expect(completed, isTrue);
        expect(
          fixture.state.status,
          wasCancelled ? LibraryStatus.cancelled : LibraryStatus.paused,
        );
      },
    );
  }
}
