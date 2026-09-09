import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "two committed root updates finish despite competing passive synchronization",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.updates.startUpdates(["a", "b"]);
      fixture.catalog.committed = true;
      await fixture.scanner.complete("C:\\a");
      await fixture.scanner.complete("C:\\b");
      await _until(() => fixture.catalog.reads.length == 1);
      expect(
        await fixture.library.refreshFromSynchronization(
          catalogRevision: BigInt.two,
        ),
        LibraryQueryUpdateOutcome.busy,
      );
      expect(fixture.catalog.reads, hasLength(1));
      fixture.catalog.complete(0);
      await _until(() => fixture.catalog.reads.length == 2);
      expect(fixture.tasks.first.phase, LibraryRootUpdatePhase.completed);
      expect(fixture.tasks.last.phase, LibraryRootUpdatePhase.refreshing);
      expect(
        await fixture.library.refreshFromSynchronization(
          catalogRevision: BigInt.two,
        ),
        LibraryQueryUpdateOutcome.busy,
      );
      fixture.catalog.complete(1);
      await _until(() => fixture.tasks.every((task) => !task.isActive));
      expect(
        fixture.tasks.map((task) => task.phase),
        everyElement(LibraryRootUpdatePhase.completed),
      );
      expect(fixture.state.roots.map((root) => root.activeScanId), [
        "new-a",
        "new-b",
      ]);
      expect(fixture.state.assets, hasLength(2));
      expect(fixture.scanner.started, ["C:\\a", "C:\\b"]);
    },
  );

  for (final userFails in [false, true]) {
    test(
      "committed refresh follows the latest user result without hiding query failure ($userFails)",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.dispose);
        fixture.updates.startUpdates(["a"]);
        fixture.catalog.committed = true;
        await fixture.scanner.complete("C:\\a");
        await _until(() => fixture.catalog.reads.length == 1);
        const requested = LibraryGalleryQuery(rootId: "b");
        final user = fixture.library.updateQuery(requested);
        expect(fixture.catalog.reads, hasLength(2));
        expect(
          await fixture.library.refreshFromSynchronization(
            catalogRevision: BigInt.two,
          ),
          LibraryQueryUpdateOutcome.busy,
        );
        if (userFails) {
          fixture.catalog.reads[1].result.completeError(
            const LibraryCatalogFailure(
              code: "user_query_failed",
              message: "fixture query failure",
            ),
          );
        } else {
          fixture.catalog.complete(1);
        }
        expect(await user, !userFails);
        fixture.catalog.complete(0);
        await _until(() => fixture.catalog.reads.length == 3);
        expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.refreshing);
        expect(
          fixture.catalog.reads.last.query.rootId,
          userFails ? isNull : "b",
        );
        fixture.catalog.complete(2);
        await _until(() => !fixture.tasks.single.isActive);
        expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.completed);
        expect(fixture.state.query.rootId, userFails ? isNull : "b");
        if (userFails) {
          final failure = fixture.state.queryActivity as LibraryQueryFailed;
          expect(failure.requestedQuery, requested);
          expect(failure.message, contains("user_query_failed"));
        } else {
          expect(fixture.state.assets.single.rootId, "b");
        }
        expect(fixture.scanner.started, ["C:\\a"]);
      },
    );
  }

  test(
    "a genuine catalog read failure stays retryable without another scan",
    () async {
      final messages = <String>[];
      final originalDebugPrint = debugPrint;
      debugPrint = (String? message, {int? wrapWidth}) {
        if (message != null) {
          messages.add(message);
        }
      };
      addTearDown(() => debugPrint = originalDebugPrint);
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.updates.startUpdates(["a"]);
      fixture.catalog.committed = true;
      await fixture.scanner.complete("C:\\a");
      await _until(() => fixture.catalog.reads.length == 1);
      fixture.catalog.reads.single.result.completeError(
        const LibraryCatalogFailure(
          code: "catalog_database_busy",
          message: "private-directory/example.jpg",
        ),
      );
      await _until(() => !fixture.tasks.single.isActive);
      expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.refreshFailed);
      expect(
        messages,
        contains("[Ame query] outcome=failed code=catalog_database_busy"),
      );
      expect(messages.join("\n"), isNot(contains("private-directory")));
      fixture.updates.retry("a");
      await _until(() => fixture.catalog.reads.length == 2);
      fixture.catalog.complete(1);
      await _until(() => !fixture.tasks.single.isActive);
      expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.completed);
      expect(fixture.scanner.started, ["C:\\a"]);
    },
  );

  test(
    "an inconsistent query snapshot fails without extra reads until an explicit retry",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.updates.startUpdates(["a"]);
      fixture.catalog.committed = true;
      await fixture.scanner.complete("C:\\a");
      await _until(() => fixture.catalog.reads.length == 1);
      expect(fixture.catalog.reads.single.snapshot.revision, BigInt.two);
      fixture.catalog.revisionOverride = BigInt.from(3);
      fixture.catalog.reads.single.result.complete(
        LibraryQuerySnapshot(
          snapshot: fixture.catalog.reads.single.snapshot,
          timeline: fixture.catalog.timeline(const LibraryGalleryQuery()),
        ),
      );
      await _until(() => !fixture.tasks.single.isActive);
      expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.refreshFailed);
      expect(fixture.tasks.single.errorMessage, isNotEmpty);
      expect(fixture.state.catalogRevision, BigInt.one);
      expect(fixture.state.isLoadingTimeline, isFalse);
      expect(fixture.catalog.reads, hasLength(1));
      fixture.catalog.revisionOverride = BigInt.from(4);
      fixture.updates.retry("a");
      await _until(() => fixture.catalog.reads.length == 2);
      fixture.catalog.complete(1);
      await _until(() => !fixture.tasks.single.isActive);
      expect(fixture.tasks.single.phase, LibraryRootUpdatePhase.completed);
      expect(fixture.state.catalogRevision, BigInt.from(4));
      expect(fixture.state.timeline?.revision, BigInt.from(4));
      expect(fixture.catalog.reads, hasLength(2));
      expect(fixture.scanner.started, ["C:\\a"]);
    },
  );
}

class _Fixture {
  _Fixture() {
    final initial = catalog.snapshot(const LibraryGalleryQuery());
    container = ProviderContainer(
      overrides: [
        libraryCatalogProvider.overrideWithValue(catalog),
        libraryScannerProvider.overrideWithValue(scanner),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            initial,
          ).copyWith(timeline: catalog.timeline(const LibraryGalleryQuery())),
        ),
      ],
    );
    library = container.read(libraryControllerProvider.notifier);
    updates = container.read(libraryUpdateControllerProvider.notifier);
  }

  final _Catalog catalog = _Catalog();
  final _Scanner scanner = _Scanner();
  late final ProviderContainer container;
  late final LibraryController library;
  late final LibraryUpdateController updates;
  LibraryState get state => container.read(libraryControllerProvider);
  List<LibraryRootUpdateTask> get tasks =>
      container.read(libraryUpdateControllerProvider).tasks;

  void dispose() {
    container.dispose();
    scanner.dispose();
  }
}

class _Read {
  _Read(this.query, this.snapshot, this.timeline);
  final LibraryGalleryQuery query;
  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;
  final Completer<LibraryQuerySnapshot> result = Completer();
}

class _Catalog implements LibraryCatalog {
  bool committed = false;
  BigInt? revisionOverride;
  final List<_Read> reads = [];
  BigInt get revision =>
      revisionOverride ?? (committed ? BigInt.two : BigInt.one);

  LibrarySnapshot snapshot(LibraryGalleryQuery query) => LibrarySnapshot(
    catalogPath: "catalog.sqlite3",
    revision: revision,
    queryId: query.rootId ?? "all",
    roots: [
      for (final id in ["a", "b"])
        LibraryRoot(
          id: id,
          path: "C:\\$id",
          displayPath: "C:\\$id",
          availability: LibraryRootAvailability.available,
          createdUnixMs: 0,
          issueCount: 0,
          assetCount: 1,
          activeScanId: "${committed ? 'new' : 'old'}-$id",
        ),
    ],
    assets: [
      for (final id in ["a", "b"])
        if (query.rootId == null || query.rootId == id)
          LibraryAsset(
            assetId: id,
            locationId: id,
            rootId: id,
            activeScanId: "${committed ? 'new' : 'old'}-$id",
            sourcePath: "C:\\$id\\image.png",
            displayPath: "image.png",
            relativePath: "image.png",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            sourceRevision: null,
            sourceGeneration: revision,
            width: 1,
            height: 1,
          ),
    ],
  );

  LibraryTimeline timeline(LibraryGalleryQuery query) => LibraryTimeline(
    revision: revision,
    queryId: query.rootId ?? "all",
    totalItems: query.rootId == null ? 2 : 1,
    buckets: const [],
  );
  void complete(int index) => reads[index].result.complete(
    LibraryQuerySnapshot(
      snapshot: reads[index].snapshot,
      timeline: reads[index].timeline,
    ),
  );

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) {
    final read = _Read(query, snapshot(query), timeline(query));
    reads.add(read);
    return read.result.future;
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) => throw StateError("Refresh must use one coherent query snapshot");

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) =>
      throw StateError("Refresh must use one coherent query snapshot");

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError();
  @override
  Future<bool> unregisterRoot(String rootId) => throw UnimplementedError();
}

class _Scanner implements LibraryScanner {
  final Map<String, StreamController<LibraryScanUpdate>> streams = {};
  final List<String> started = [];
  Future<void> complete(String path) async {
    final stream = streams.remove(path)!;
    stream.add(
      const LibraryScanCompleted(
        assetCount: 1,
        issueCount: 0,
        catalogPath: "catalog.sqlite3",
        wasLimited: false,
      ),
    );
    await stream.close();
  }

  void dispose() {
    for (final stream in streams.values) {
      unawaited(stream.close());
    }
    streams.clear();
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    // All fixture streams are closed by completion or fixture disposal.
    // ignore: close_sinks
    final stream = StreamController<LibraryScanUpdate>();
    streams[rootPath] = stream;
    started.add(rootPath);
    return stream.stream;
  }

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;
  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;
  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) => throw UnimplementedError();
  @override
  Future<void> cancelRetainedScan(String scanId) async {}
  @override
  bool cancel(String scanId) => false;
  @override
  bool pause(String scanId) => false;
  @override
  bool suspend(String scanId) => false;
}

Future<void> _until(bool Function() condition) async {
  for (var attempt = 0; attempt < 50; attempt += 1) {
    if (condition()) {
      return;
    }
    await Future<void>.delayed(Duration.zero);
  }
  fail("The controlled query transition did not reach its required boundary");
}
