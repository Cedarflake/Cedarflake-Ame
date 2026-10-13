import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets(
    "sidebar import from settings opens the existing gallery folder",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fixture = _Fixture();
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: fixture.container,
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-sidebar-settings")));
      await tester.pump();
      expect(find.byType(LibraryGalleryWall), findsNothing);
      await tester.tap(find.byKey(const Key("library-sidebar-import")));
      await tester.pump();
      fixture.scanner.streams.single.add(fixture.coverage);
      unawaited(fixture.scanner.streams.single.close());
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.folderRelativePath, "旅行/海边");
      expect(find.byType(LibraryGalleryWall), findsOneWidget);
      expect(find.byType(PendingLibrarySourceTile), findsNothing);
      expect(fixture.state.roots, [otherPublishedRoot]);
      expect(fixture.state.taskKind, isNull);
      expect(tester.takeException(), isNull);
    },
  );

  test(
    "picker selection opens the covered folder without import feedback or source refresh",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.updateQuery(
        const LibraryGalleryQuery(searchText: "旅行"),
      );
      await fixture.controller.chooseDirectoryAndScan();
      final before = fixture.catalog.firstLoads;
      await fixture.finishCoverage();
      expect(fixture.state.query.rootId, "other");
      expect(fixture.state.query.folderRelativePath, "旅行/海边");
      expect(fixture.state.query.includeDescendants, isTrue);
      expect(fixture.state.query.searchText, "旅行");
      expect(fixture.catalog.firstLoads, before + 1);
      expect(fixture.state.roots, [otherPublishedRoot]);
      expect(fixture.state.primaryScanSnapshot.taskKind, isNull);
      expect(fixture.state.hasRetainedScan, isFalse);
      expect(fixture.scanner.startedRoots, hasLength(1));
      expect(fixture.picker.calls, 1);
    },
  );

  test(
    "later A-B-A navigation is not replaced by delayed folder coverage",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndScan();
      final original = fixture.state.query;
      await fixture.controller.updateQuery(
        original.copyWith(searchText: "later"),
      );
      await fixture.controller.updateQuery(original);
      final before = fixture.catalog.firstLoads;
      await fixture.finishCoverage();
      expect(fixture.state.query, original);
      expect(fixture.catalog.firstLoads, before);
      expect(fixture.state.taskKind, isNull);
    },
  );

  test(
    "folder read failure retries only display and preserves the existing source",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndScan();
      fixture.catalog.loadFailure = StateError("folder load unavailable");
      await fixture.finishCoverage();
      expect(fixture.state.queryActivity, isA<LibraryQueryFailed>());
      final failed = fixture.state.queryActivity as LibraryQueryFailed;
      expect(failed.requestedQuery.folderRelativePath, "旅行/海边");
      expect(fixture.state.taskKind, isNull);
      fixture.catalog.loadFailure = null;
      expect(
        await fixture.controller.updateQuery(
          failed.requestedQuery,
          forceRefresh: true,
        ),
        isTrue,
      );
      expect(fixture.state.query.folderRelativePath, "旅行/海边");
      expect(fixture.scanner.startedRoots, hasLength(1));
      expect(fixture.state.roots, [otherPublishedRoot]);
    },
  );

  test(
    "the next picker waits for covered-folder navigation retirement",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndScan();
      final pending = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      final covered = fixture.coverage;
      await fixture.finishCoverage();
      final next = fixture.controller.chooseDirectoryAndScan();
      await flushRetainedScanMicrotasks();
      expect(fixture.picker.calls, 1);
      pending.complete(
        fixture.catalog.snapshot(
          fixture.state.query.copyWith(
            rootId: covered.rootId,
            folderRelativePath: covered.relativePath,
            includeDescendants: true,
          ),
        ),
      );
      await next;
      expect(fixture.picker.calls, 2);
      expect(fixture.scanner.startedRoots, hasLength(2));
    },
  );

  test("cancel before coverage prevents navigation", () async {
    final fixture = _Fixture();
    await fixture.ready();
    await fixture.controller.chooseDirectoryAndScan();
    final original = fixture.state.query;
    await fixture.controller.cancelScan();
    await fixture.finishCoverage();
    expect(fixture.state.query, original);
    expect(fixture.state.status, LibraryStatus.cancelled);
    expect(fixture.state.hasRetainedScan, isFalse);
  });

  for (final foreign in [false, true]) {
    test("invalid covered terminal is rejected (foreign: $foreign)", () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndScan();
      final original = fixture.state.query;
      if (!foreign) {
        fixture.scanner.streams.single.add(
          LibraryScanStarted(
            scanId: fixture.state.scanId!,
            rootPath: "C:\\Other\\旅行\\海边",
          ),
        );
        await flushRetainedScanMicrotasks();
      }
      fixture.scanner.streams.single.add(
        LibraryScanAlreadyCovered(
          scanId: foreign ? "foreign" : fixture.state.scanId!,
          rootId: "other",
          relativePath: "旅行/海边",
        ),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      expect(fixture.state.query, original);
      expect(fixture.state.status, LibraryStatus.failed);
    });
  }

  test("disposal retires pending folder publication", () async {
    final fixture = _Fixture();
    await fixture.ready();
    await fixture.controller.chooseDirectoryAndScan();
    final pending = fixture.catalog.pendingLoad = Completer<LibrarySnapshot>();
    await fixture.finishCoverage();
    final retained = fixture.state;
    fixture.dispose();
    pending.complete(
      fixture.catalog.snapshot(
        const LibraryGalleryQuery(rootId: "other", folderRelativePath: "旅行/海边"),
      ),
    );
    await flushRetainedScanMicrotasks();
    expect(retained.query.folderRelativePath, isNull);
  });
}

class _Fixture {
  _Fixture() {
    scanner.checkpoint = null;
    catalog = RetainedScanCatalog(scanner);
    catalog.roots
      ..clear()
      ..add(otherPublishedRoot);
    container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
        directoryPickerProvider.overrideWithValue(picker),
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
  final picker = _Picker();
  late final RetainedScanCatalog catalog;
  late final ProviderContainer container;
  bool _isDisposed = false;
  LibraryController get controller =>
      container.read(libraryControllerProvider.notifier);
  LibraryState get state => container.read(libraryControllerProvider);
  LibraryScanAlreadyCovered get coverage => LibraryScanAlreadyCovered(
    scanId: state.scanId!,
    rootId: "other",
    relativePath: "旅行/海边",
  );

  Future<void> ready() async {
    controller;
    await flushRetainedScanMicrotasks();
  }

  Future<void> finishCoverage() async {
    scanner.streams.single.add(coverage);
    await scanner.streams.single.close();
    await flushRetainedScanMicrotasks();
  }

  void dispose() {
    if (_isDisposed) return;
    _isDisposed = true;
    container.dispose();
    scanner.dispose();
  }
}

class _Picker implements DirectoryPicker {
  int calls = 0;
  @override
  Future<String?> pickDirectory() async {
    calls += 1;
    return "C:\\Other\\旅行\\海边";
  }
}

class _Scanner extends RetainedScanScanner {
  final streams = <StreamController<LibraryScanUpdate>>[];

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedRoots.add(rootPath);
    final stream = StreamController<LibraryScanUpdate>();
    streams.add(stream);
    return stream.stream;
  }

  @override
  void dispose() {
    for (final stream in streams) {
      if (!stream.isClosed) unawaited(stream.close());
    }
    super.dispose();
  }
}
