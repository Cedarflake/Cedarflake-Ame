import "dart:async";

import "package:cedarflake_ame/features/storage/presentation/storage_snapshot_coordinator.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("same-version requests share one read and one completion", () async {
    final reader = _Reader();
    final coordinator = StorageSnapshotCoordinator(reader);
    addTearDown(coordinator.dispose);
    final first = coordinator.refresh();
    final second = coordinator.refresh();
    expect(identical(first, second), isTrue);
    expect(reader.reads, hasLength(1));
    reader.reads.single.complete(StorageSnapshotReadOutcome.settled);
    await first;
    expect(coordinator.isBusy, isFalse);
    final next = coordinator.refresh();
    expect(reader.reads, hasLength(2));
    reader.reads.last.complete(StorageSnapshotReadOutcome.settled);
    await next;
  });

  for (final changesEpoch in [false, true]) {
    test(
      "new freshness requirements coalesce behind the in-flight read (epoch: $changesEpoch)",
      () async {
        final reader = _Reader();
        final coordinator = StorageSnapshotCoordinator(reader);
        addTearDown(coordinator.dispose);
        final first = coordinator.refresh();
        reader.version = changesEpoch
            ? const StorageSnapshotVersion(1, 0)
            : const StorageSnapshotVersion(0, 1);
        final second = coordinator.refresh();
        reader.version = changesEpoch
            ? const StorageSnapshotVersion(2, 0)
            : const StorageSnapshotVersion(0, 2);
        final latest = coordinator.refresh();
        expect(identical(first, second), isTrue);
        expect(identical(second, latest), isTrue);
        expect(reader.reads, hasLength(1));
        var completed = false;
        unawaited(latest.then((_) => completed = true));
        reader.reads.first.complete(StorageSnapshotReadOutcome.settled);
        await Future<void>.value();
        expect(completed, isFalse);
        expect(reader.reads, hasLength(2));
        expect(reader.versions.last, reader.version);
        expect(identical(coordinator.refresh(), latest), isTrue);
        reader.reads.last.complete(StorageSnapshotReadOutcome.settled);
        await latest;
        expect(completed, isTrue);
        expect(reader.reads, hasLength(2));
      },
    );
  }

  test(
    "a suspended command retains one refresh obligation until wake",
    () async {
      final reader = _Reader()
        ..availability = StorageSnapshotReadAvailability.suspended;
      final coordinator = StorageSnapshotCoordinator(reader);
      addTearDown(coordinator.dispose);
      final first = coordinator.refresh();
      expect(identical(first, coordinator.refresh()), isTrue);
      expect(reader.reads, isEmpty);
      reader.version = const StorageSnapshotVersion(1, 0);
      reader.availability = StorageSnapshotReadAvailability.available;
      coordinator.wake();
      coordinator.wake();
      expect(reader.reads, hasLength(1));
      expect(reader.versions.single, reader.version);
      reader.reads.single.complete(StorageSnapshotReadOutcome.settled);
      await first;
    },
  );

  test(
    "a read superseded by a pending command waits for its available epoch",
    () async {
      final reader = _Reader();
      final coordinator = StorageSnapshotCoordinator(reader);
      addTearDown(coordinator.dispose);
      final refreshing = coordinator.refresh();
      reader.version = const StorageSnapshotVersion(1, 0);
      reader.availability = StorageSnapshotReadAvailability.suspended;
      reader.reads.single.complete(StorageSnapshotReadOutcome.superseded);
      await Future<void>.value();
      expect(reader.reads, hasLength(1));
      expect(coordinator.isBusy, isTrue);
      reader.version = const StorageSnapshotVersion(2, 0);
      reader.availability = StorageSnapshotReadAvailability.available;
      coordinator.wake();
      expect(reader.reads, hasLength(2));
      reader.reads.last.complete(StorageSnapshotReadOutcome.settled);
      await refreshing;
    },
  );

  for (final suspended in [false, true]) {
    test(
      "disposal settles waiting requests without awaiting the backend (suspended: $suspended)",
      () async {
        final reader = _Reader();
        if (suspended) {
          reader.availability = StorageSnapshotReadAvailability.suspended;
        }
        final coordinator = StorageSnapshotCoordinator(reader);
        final refreshing = coordinator.refresh();
        coordinator.dispose();
        await refreshing;
        expect(coordinator.isBusy, isFalse);
        await coordinator.refresh();
        if (!suspended) {
          reader.reads.single.complete(StorageSnapshotReadOutcome.superseded);
          await Future<void>.value();
        }
        expect(reader.reads, hasLength(suspended ? 0 : 1));
      },
    );
  }
}

class _Reader implements StorageSnapshotReader {
  StorageSnapshotVersion version = const StorageSnapshotVersion(0, 0);
  StorageSnapshotReadAvailability availability =
      StorageSnapshotReadAvailability.available;
  final versions = <StorageSnapshotVersion>[];
  final reads = <Completer<StorageSnapshotReadOutcome>>[];

  @override
  StorageSnapshotReadContext get storageSnapshotReadContext =>
      StorageSnapshotReadContext(version, availability);

  @override
  Future<StorageSnapshotReadOutcome> readStorageSnapshot(
    StorageSnapshotVersion version,
  ) {
    versions.add(version);
    final completion = Completer<StorageSnapshotReadOutcome>();
    reads.add(completion);
    return completion.future;
  }
}
