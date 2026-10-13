import "dart:async";

import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

const _childQuery = LibraryGalleryQuery(
  rootId: "other",
  folderRelativePath: "海边",
  includeDescendants: false,
  searchText: "sun",
  sortKey: LibraryGallerySortKey.fileName,
  sortDirection: LibraryGallerySortDirection.ascending,
);
const _consolidation = LibraryRootConsolidation(
  destinationRootId: "parent",
  roots: [LibraryConsolidatedRoot(previousRootId: "other", relativePath: "旅行")],
);

void main() {
  for (final deliverTerminal in [false, true]) {
    test(
      "unresolved completion survives dismiss and another import (terminal: $deliverTerminal)",
      () async {
        final fixture = _Fixture();
        await fixture.begin();
        fixture.catalog.loadFailure = StateError("first result read failed");
        await fixture.finish(deliverTerminal: deliverTerminal);
        expect(fixture.state.status, LibraryStatus.failed);
        expect(
          fixture.state.primaryScanSnapshot.hasUnresolvedPublication,
          isTrue,
        );
        fixture.controller.dismissTaskFeedback();
        await fixture.controller.chooseDirectoryAndScan();
        await fixture.controller.scanDirectory("C:\\Another");
        expect(
          fixture.state.primaryScanSnapshot.hasUnresolvedPublication,
          isTrue,
        );
        expect(fixture.scanner.startedRoots, hasLength(1));
        fixture.catalog.loadFailure = null;
        await fixture.controller.retry();
        await flushRetainedScanMicrotasks();
        expect(fixture.state.status, LibraryStatus.completed);
        expect(fixture.state.query.rootId, "parent");
        expect(fixture.state.query.folderRelativePath, "旅行/海边");
        expect(
          fixture.state.primaryScanSnapshot.hasUnresolvedPublication,
          isFalse,
        );
        expect(fixture.scanner.startedRoots, hasLength(1));
        fixture.controller.dismissTaskFeedback();
        expect(fixture.state.scanId, isNull);
      },
    );
  }

  test(
    "a lost completion event recovers folder continuity from committed roots",
    () async {
      final fixture = _Fixture();
      await fixture.begin();
      await fixture.finish(deliverTerminal: false);
      expect(fixture.state.status, LibraryStatus.completed);
      expect(fixture.state.query.rootId, "parent");
      expect(fixture.state.query.folderRelativePath, "旅行/海边");
      expect(fixture.scanner.startedRoots, hasLength(1));
    },
  );

  test(
    "parent commit preserves the current child folder and query options",
    () async {
      final fixture = _Fixture();
      await fixture.begin();
      await fixture.finish();
      expect(
        fixture.state.query,
        _childQuery.copyWith(rootId: "parent", folderRelativePath: "旅行/海边"),
      );
      expect(fixture.state.status, LibraryStatus.completed);
      expect(fixture.catalog.requestedQueries.last, fixture.state.query);
      expect(fixture.state.roots.single.id, "parent");
    },
  );

  for (final later in [
    const LibraryGalleryQuery(searchText: "all"),
    const LibraryGalleryQuery(rootId: "peer", searchText: "later"),
    _childQuery.copyWith(folderRelativePath: "山间", searchText: "later"),
  ]) {
    test(
      "later navigation retains its scope after parent commit: ${later.rootId}/${later.folderRelativePath}",
      () async {
        final fixture = _Fixture();
        await fixture.begin();
        await fixture.controller.updateQuery(later);
        await fixture.finish();
        final expected = later.rootId == "other"
            ? later.copyWith(rootId: "parent", folderRelativePath: "旅行/山间")
            : later;
        expect(fixture.state.query, expected);
      },
    );
  }

  test(
    "failed display refresh retains consolidation for retry without another scan",
    () async {
      final fixture = _Fixture();
      await fixture.begin();
      fixture.catalog.loadFailure = StateError("display read unavailable");
      await fixture.finish();
      expect(fixture.state.status, LibraryStatus.failed);
      expect(fixture.state.query, _childQuery);
      fixture.catalog.loadFailure = null;
      await fixture.controller.retry();
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.completed);
      expect(fixture.state.query.rootId, "parent");
      expect(fixture.state.query.folderRelativePath, "旅行/海边");
      expect(fixture.scanner.startedRoots, hasLength(1));
    },
  );

  test(
    "later user input supersedes an in-flight consolidated folder read",
    () async {
      final fixture = _Fixture();
      await fixture.begin();
      final pending = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      await fixture.finish();
      final later = _childQuery.copyWith(
        rootId: "peer",
        folderRelativePath: null,
      );
      await fixture.controller.updateQuery(later);
      pending.complete(
        fixture.catalog.snapshot(
          _childQuery.copyWith(rootId: "parent", folderRelativePath: "旅行/海边"),
        ),
      );
      await flushRetainedScanMicrotasks();
      expect(fixture.state.query, later);
      expect(fixture.state.status, LibraryStatus.completed);
    },
  );

  test(
    "disposal prevents a pending consolidated view from publishing",
    () async {
      final fixture = _Fixture();
      await fixture.begin();
      final pending = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      await fixture.finish();
      final retained = fixture.state;
      fixture.dispose();
      pending.complete(fixture.catalog.snapshot(_childQuery));
      await flushRetainedScanMicrotasks();
      expect(retained.query, _childQuery);
    },
  );
}

class _Fixture {
  _Fixture() {
    scanner.checkpoint = null;
    catalog = _Catalog(scanner);
    catalog.roots
      ..clear()
      ..add(
        const LibraryRoot(
          id: "other",
          path: "C:\\New\\旅行",
          displayPath: "C:\\New\\旅行",
          activeScanId: "published-child",
          createdUnixMs: 1,
          assetCount: 6,
          issueCount: 0,
          availability: LibraryRootAvailability.available,
        ),
      );
    container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
        directoryPickerProvider.overrideWithValue(RetainedScanPicker()),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            catalog.snapshot(const LibraryGalleryQuery()),
          ),
        ),
      ],
    );
    addTearDown(dispose);
  }

  final scanner = _Scanner();
  late final _Catalog catalog;
  late final ProviderContainer container;
  bool _isDisposed = false;

  LibraryController get controller =>
      container.read(libraryControllerProvider.notifier);
  LibraryState get state => container.read(libraryControllerProvider);

  Future<void> begin() async {
    controller;
    await flushRetainedScanMicrotasks();
    await controller.updateQuery(_childQuery);
    await controller.chooseDirectoryAndScan();
    scanner.events.add(
      LibraryScanStarted(scanId: state.scanId!, rootPath: "C:\\New"),
    );
    await flushRetainedScanMicrotasks();
  }

  Future<void> finish({bool deliverTerminal = true}) async {
    catalog.roots
      ..clear()
      ..add(
        LibraryRoot(
          id: "parent",
          path: "C:\\New",
          displayPath: "C:\\New",
          activeScanId: state.scanId,
          createdUnixMs: 1,
          assetCount: 6,
          issueCount: 0,
          availability: LibraryRootAvailability.available,
        ),
      );
    if (deliverTerminal) {
      scanner.events.add(
        const LibraryScanCompleted(
          assetCount: 6,
          issueCount: 0,
          catalogPath: "catalog.sqlite3",
          wasLimited: false,
          rootConsolidation: _consolidation,
        ),
      );
    }
    await scanner.events.close();
    await flushRetainedScanMicrotasks();
  }

  void dispose() {
    if (_isDisposed) return;
    _isDisposed = true;
    container.dispose();
    scanner.dispose();
  }
}

class _Scanner extends RetainedScanScanner {
  final events = StreamController<LibraryScanUpdate>();

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedRoots.add(rootPath);
    return events.stream;
  }

  @override
  void dispose() {
    if (!events.isClosed) unawaited(events.close());
    super.dispose();
  }
}

class _Catalog extends RetainedScanCatalog {
  _Catalog(super.scanner);

  final requestedQueries = <LibraryGalleryQuery>[];

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) {
    requestedQueries.add(query);
    return super.load(
      maxItems: maxItems,
      query: query,
      after: after,
      before: before,
    );
  }
}
