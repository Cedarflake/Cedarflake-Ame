import "dart:async";

import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("early cancellation is observed before a consumer attaches", () async {
    final reader = _Reader();
    final scheduler = LibrarySourceReadScheduler(reader: reader);
    final first = scheduler.request(_asset("first"));
    final pending = scheduler.request(_asset("pending"));
    pending.cancel();
    pending.cancel();
    await Future<void>.delayed(Duration.zero);
    await expectLater(pending.lease, throwsA(isA<LibrarySourceReadFailure>()));
    reader.responses.first.complete(_Lease("first")..closed.complete());
    await Future<void>.delayed(Duration.zero);
    await expectLater(first.lease, throwsA(isA<LibrarySourceReadFailure>()));
    expect(reader.opened, ["first"]);
  });

  test(
    "only the latest waiting source opens after the active read releases",
    () async {
      final reader = _Reader();
      final scheduler = LibrarySourceReadScheduler(reader: reader);
      final first = scheduler.request(_asset("first"));
      final firstResult = expectLater(
        first.lease,
        throwsA(isA<LibrarySourceReadFailure>()),
      );
      final second = scheduler.request(_asset("second"));
      final secondResult = expectLater(
        second.lease,
        throwsA(isA<LibrarySourceReadFailure>()),
      );
      final latest = scheduler.request(_asset("latest"));
      expect(reader.opened, ["first"]);
      final oldLease = _Lease("first");
      reader.responses.first.complete(oldLease);
      await Future<void>.delayed(Duration.zero);
      expect(oldLease.closeCount, 1);
      expect(reader.opened, ["first"]);
      oldLease.closed.complete();
      await firstResult;
      await secondResult;
      expect(reader.opened, ["first", "latest"]);
      final newestLease = _Lease("latest");
      reader.responses.last.complete(newestLease);
      final acquired = await latest.lease;
      final closing = acquired.close();
      newestLease.closed.complete();
      await closing;
    },
  );

  test("cancellation never closes an active buffer lease early", () async {
    final reader = _Reader();
    final scheduler = LibrarySourceReadScheduler(reader: reader);
    final first = scheduler.request(_asset("first"));
    final underlying = _Lease("first");
    reader.responses.first.complete(underlying);
    final lease = await first.lease;
    final latest = scheduler.request(_asset("latest"));
    first.cancel();
    expect(first.isCancelled, isTrue);
    expect(underlying.closeCount, 0);
    expect(reader.opened, ["first"]);
    final closed = lease.close();
    expect(underlying.closeCount, 1);
    expect(identical(lease.close(), closed), isTrue);
    underlying.closed.complete();
    await closed;
    expect(reader.opened, ["first", "latest"]);
    latest.cancel();
    final outcome = expectLater(
      latest.lease,
      throwsA(isA<LibrarySourceReadFailure>()),
    );
    final pending = _Lease("latest")..closed.complete();
    reader.responses.last.complete(pending);
    await outcome;
    expect(pending.closeCount, 1);
  });

  test(
    "failed acquisition drains the latest request without replaying stale work",
    () async {
      final reader = _Reader();
      final scheduler = LibrarySourceReadScheduler(reader: reader);
      final first = scheduler.request(_asset("first"));
      final failed = expectLater(first.lease, throwsA(isA<StateError>()));
      final latest = scheduler.request(_asset("latest"));
      reader.responses.first.completeError(StateError("source unavailable"));
      await failed;
      expect(reader.opened, ["first", "latest"]);
      final lease = _Lease("latest")..closed.complete();
      reader.responses.last.complete(lease);
      await (await latest.lease).close();
    },
  );

  test(
    "closing a waiting viewer discards it without opening its source",
    () async {
      final reader = _Reader();
      final scheduler = LibrarySourceReadScheduler(reader: reader);
      final first = scheduler.request(_asset("first"));
      final failed = expectLater(
        first.lease,
        throwsA(isA<LibrarySourceReadFailure>()),
      );
      final waiting = scheduler.request(_asset("waiting"));
      final cancelled = expectLater(
        waiting.lease,
        throwsA(isA<LibrarySourceReadFailure>()),
      );
      waiting.cancel();
      reader.responses.first.complete(_Lease("first")..closed.complete());
      await failed;
      await cancelled;
      expect(reader.opened, ["first"]);
    },
  );

  test("release failure cannot strand the next viewer intent", () async {
    final reader = _Reader();
    final scheduler = LibrarySourceReadScheduler(reader: reader);
    final first = scheduler.request(_asset("first"));
    final underlying = _Lease("first");
    reader.responses.first.complete(underlying);
    final lease = await first.lease;
    final latest = scheduler.request(_asset("latest"));
    final closing = expectLater(lease.close(), throwsA(isA<StateError>()));
    underlying.closed.completeError(StateError("release failed"));
    await closing;
    expect(reader.opened, ["first", "latest"]);
    reader.responses.last.complete(_Lease("latest")..closed.complete());
    await (await latest.lease).close();
  });
}

class _Reader implements LibrarySourceReader {
  final List<String> opened = [];
  final List<Completer<LibrarySourceReadLease>> responses = [];

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) {
    opened.add(asset.locationId);
    final response = Completer<LibrarySourceReadLease>();
    responses.add(response);
    return response.future;
  }
}

class _Lease implements LibrarySourceReadLease {
  _Lease(this.sourcePath);

  @override
  final String sourcePath;
  final Completer<void> closed = Completer();
  int closeCount = 0;

  @override
  Future<void> close() {
    closeCount += 1;
    return closed.future;
  }
}

LibraryAsset _asset(String id) => LibraryAsset(
  assetId: id,
  locationId: id,
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "$id.png",
  displayPath: "$id.png",
  relativePath: "$id.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 1,
  height: 1,
);
