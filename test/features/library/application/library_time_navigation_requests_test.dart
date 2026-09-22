import "dart:async";

import "package:cedarflake_ame/features/library/application/library_time_navigation_requests.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("pending and active callers share the exact target", (
    tester,
  ) async {
    final host = _NavigationHost();
    addTearDown(host.requests.dispose);
    final first = host.jump(20);
    expect(host.jump(20), same(first));
    await tester.pump();
    expect(host.jump(20), same(first));
    expect(host.reads, hasLength(1));
    host.reads.single.result.complete(true);
    await tester.pump();
    expect(await first, isTrue);
    expect(host.isLoading, isFalse);
    expect(host.requests.hasVisibleRangeLoading, isFalse);
    expect(host.requests.retainsVisibleRange(start: 20, end: 21), isTrue);
    expect(host.requests.retainsVisibleRange(start: 0, end: 20), isFalse);
  });

  testWidgets(
    "new intent retires queued work and old cleanup keeps new loading",
    (tester) async {
      final host = _NavigationHost();
      addTearDown(host.requests.dispose);
      final first = host.jump(20);
      await tester.pump();
      final supersededPending = host.jump(40);
      final latest = host.jump(80);
      expect(await supersededPending, isFalse);
      expect(host.reads, hasLength(1));
      final old = host.reads.single.request;
      host.reads.first.result.complete(true);
      await tester.pump();
      expect(await first, isFalse);
      expect(host.reads.map((read) => read.request.globalItemOffset), [20, 80]);
      expect(host.requests.hasVisibleRangeLoading, isTrue);
      expect(host.visibleLoading, everyElement(isTrue));
      expect(host.isLoading, isTrue);
      final releases = host.loadingReleases;
      host.requests.releaseLoading(old);
      expect(host.loadingReleases, releases);
      expect(host.isLoading, isTrue);
      host.reads.last.result.complete(true);
      await tester.pump();
      expect(await latest, isTrue);
      expect(host.visibleLoading.last, isFalse);
    },
  );

  testWidgets("explicit ownership rejects passive work until cancelled", (
    tester,
  ) async {
    final host = _NavigationHost();
    addTearDown(host.requests.dispose);
    final explicit = host.jump(80);
    expect(await host.jump(10, explicit: false), isFalse);
    await tester.pump();
    expect(host.reads, hasLength(1));
    expect(host.requests.cancel(), isTrue);
    expect(await explicit, isFalse);
    final passive = host.jump(10, explicit: false);
    expect(host.reads, hasLength(1));
    host.reads.first.result.complete(false);
    await tester.pump();
    expect(host.reads.last.request.globalItemOffset, 10);
    host.reads.last.result.complete(true);
    await tester.pump();
    expect(await passive, isTrue);
    expect(host.requests.hasVisibleRangeOwner, isFalse);
  });

  testWidgets("passive ranges retire obsolete pending and active targets", (
    tester,
  ) async {
    final host = _NavigationHost();
    addTearDown(host.requests.dispose);
    final active = host.jump(60, explicit: false);
    await tester.pump();
    final pending = host.jump(80, explicit: false);
    host.requests.retainPassiveInRange(start: 0, end: 10);
    expect(await pending, isFalse);
    expect(host.requests.accepts(host.reads.single.request), isFalse);
    host.reads.single.result.complete(true);
    await tester.pump();
    expect(await active, isFalse);
    expect(host.reads, hasLength(1));
  });

  for (final oldResult in [true, false]) {
    testWidgets(
      "query supersession ignores old result=$oldResult and releases only its owner",
      (tester) async {
        final host = _NavigationHost();
        addTearDown(host.requests.dispose);
        final oldFuture = host.jump(20);
        await tester.pump();
        final oldRequest = host.reads.single.request;
        host.requests.supersedeQuery();
        host.queryId = "replacement";
        expect(await oldFuture, isFalse);
        final replacement = host.jump(80);
        host.reads.first.result.complete(oldResult);
        await tester.pump();
        expect(host.requests.accepts(oldRequest), isFalse);
        expect(host.loadingReleases, 0);
        expect(host.requests.hasVisibleRangeLoading, isTrue);
        host.reads.last.result.complete(true);
        await tester.pump();
        expect(await replacement, isTrue);
        expect(host.loadingReleases, 1);
      },
    );
  }

  testWidgets("failed current navigation retires explicit ownership", (
    tester,
  ) async {
    final host = _NavigationHost();
    addTearDown(host.requests.dispose);
    final failed = host.jump(20);
    await tester.pump();
    host.reads.single.result.complete(false);
    await tester.pump();
    expect(await failed, isFalse);
    expect(host.requests.hasVisibleRangeOwner, isFalse);
    expect(host.isLoading, isFalse);
    expect(host.requests.hasVisibleRangeLoading, isFalse);
    final retry = host.jump(20);
    await tester.pump();
    host.reads.last.result.complete(true);
    await tester.pump();
    expect(await retry, isTrue);
  });

  testWidgets("blocked admission waits for the existing retry boundary", (
    tester,
  ) async {
    final host = _NavigationHost()..isBlocked = true;
    addTearDown(host.requests.dispose);
    final future = host.jump(20);
    await tester.pump();
    expect(host.reads, isEmpty);
    host.isBlocked = false;
    await tester.pump(const Duration(milliseconds: 119));
    expect(host.reads, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(host.reads, hasLength(1));
    host.reads.single.result.complete(true);
    await tester.pump();
    expect(await future, isTrue);
  });

  testWidgets("incompatible queued target is settled before any catalog read", (
    tester,
  ) async {
    final host = _NavigationHost();
    addTearDown(host.requests.dispose);
    final future = host.jump(20);
    host.queryId = "changed";
    await tester.pump();
    expect(await future, isFalse);
    expect(host.reads, isEmpty);
    expect(host.requests.hasVisibleRangeLoading, isFalse);
    expect(host.requests.hasVisibleRangeOwner, isFalse);
  });

  testWidgets(
    "disposal settles active and queued callers before backend completion",
    (tester) async {
      final host = _NavigationHost();
      final active = host.jump(20);
      await tester.pump();
      final queued = host.jump(80);
      final notifications = host.visibleLoading.length;
      host.requests.dispose();
      expect(await active, isFalse);
      expect(await queued, isFalse);
      expect(await host.jump(10), isFalse);
      host.reads.single.result.complete(false);
      await tester.pump();
      expect(host.visibleLoading, hasLength(notifications));
      expect(host.loadingReleases, 0);
      expect(host.reads, hasLength(1));
    },
  );

  testWidgets("disposal cancels the pending admission timer", (tester) async {
    final host = _NavigationHost()..isBlocked = true;
    final future = host.jump(20);
    await tester.pump();
    host.requests.dispose();
    expect(await future, isFalse);
    host.isBlocked = false;
    await tester.pump(const Duration(seconds: 1));
    expect(host.reads, isEmpty);
  });
}

class _NavigationHost {
  _NavigationHost() {
    requests = LibraryTimeNavigationRequests(
      cannotPublish: () => false,
      isBlocked: () => isBlocked,
      isCompatible: (request) => request.timeline.queryId == queryId,
      load: _load,
      onVisibleRangeLoading: (_) =>
          visibleLoading.add(requests.hasVisibleRangeLoading),
      onTimeAnchorLoadingReleased: () {
        loadingReleases += 1;
        isLoading = false;
      },
    );
  }

  late final LibraryTimeNavigationRequests requests;
  final reads = <_Read>[];
  final visibleLoading = <bool>[];
  var isBlocked = false;
  var isLoading = false;
  var loadingReleases = 0;
  var queryId = "initial";

  Future<bool> jump(int offset, {bool explicit = true}) {
    final timeline = LibraryTimeline(
      revision: BigInt.one,
      queryId: queryId,
      totalItems: 100,
      buckets: const [],
    );
    return requests.request(
      query: const LibraryGalleryQuery(),
      timeline: timeline,
      anchor: LibraryTimeAnchor(
        revision: timeline.revision,
        queryId: queryId,
        itemOffset: offset,
      ),
      globalItemOffset: offset,
      ownsVisibleRange: explicit,
      needsVisibleRangeLoading: explicit,
    );
  }

  Future<bool> _load(LibraryTimeNavigationRequest request) async {
    final read = _Read(request);
    reads.add(read);
    requests.beginLoading(request);
    isLoading = true;
    try {
      return await read.result.future && requests.accepts(request);
    } finally {
      requests.releaseLoading(request);
    }
  }
}

class _Read {
  _Read(this.request);

  final LibraryTimeNavigationRequest request;
  final result = Completer<bool>();
}
