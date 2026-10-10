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

void main() {
  for (final didStart in [false, true]) {
    test(
      "ordinary identity recovery refreshes its source before cancellation (Started: $didStart)",
      () async {
        final fixture = _Fixture();
        await fixture.ready();
        await fixture.controller.scanDirectory("C:\\Moved");
        final scanId = fixture.state.scanId!;
        expect(fixture.state.taskKind, LibraryTaskKind.import);
        fixture.movePublishedRoot();
        if (didStart) {
          fixture.scanner.streams.single.add(
            LibraryScanStarted(scanId: scanId, rootPath: "C:\\Moved"),
          );
          await flushRetainedScanMicrotasks();
          expect(fixture.state.roots.last.path, "C:\\Moved");
          expect(fixture.state.taskKind, LibraryTaskKind.update);
        }
        fixture.scanner.streams.single.add(
          const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
        );
        await fixture.scanner.streams.single.close();
        await flushRetainedScanMicrotasks();
        expect(fixture.state.roots.last.path, "C:\\Moved");
        expect(fixture.state.status, LibraryStatus.cancelled);
        expect(fixture.state.taskKind, LibraryTaskKind.update);
        await fixture.controller.retry();
        expect(fixture.scanner.startedRoots, ["C:\\Moved", "C:\\Moved"]);
        expect(fixture.scanner.relocations, isEmpty);
      },
    );
  }

  test(
    "Started refresh keeps ownership through immediate cancellation",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.scanDirectory("C:\\Moved");
      final scanId = fixture.state.scanId!;
      fixture.movePublishedRoot();
      final refresh = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      fixture.scanner.streams.single.add(
        LibraryScanStarted(scanId: scanId, rootPath: "C:\\Moved"),
      );
      await flushRetainedScanMicrotasks();
      fixture.scanner.streams.single.add(
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      final retry = fixture.controller.retry();
      await flushRetainedScanMicrotasks();
      expect(fixture.scanner.startedRoots, hasLength(1));
      refresh.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
      await retry;
      expect(fixture.state.roots.last.path, "C:\\Moved");
      expect(fixture.state.taskKind, LibraryTaskKind.update);
      expect(fixture.scanner.startedRoots, hasLength(2));
    },
  );

  for (final chooseAgain in [false, true]) {
    test(
      "${chooseAgain ? 'picker' : 'retry'} waits for terminal source refresh ownership",
      () async {
        final fixture = _Fixture();
        await fixture.ready();
        await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
        final refresh = fixture.catalog.pendingLoad =
            Completer<LibrarySnapshot>();
        fixture.scanner.streams.single.add(
          const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
        );
        await fixture.scanner.streams.single.close();
        await flushRetainedScanMicrotasks();
        final next = chooseAgain
            ? fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot)
            : fixture.controller.retry();
        await flushRetainedScanMicrotasks();
        expect(fixture.scanner.relocations, hasLength(1));
        expect(fixture.picker.calls, 1);
        fixture.catalog.pendingLoad = null;
        refresh.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
        await next;
        expect(fixture.scanner.relocations, hasLength(2));
        expect(fixture.picker.calls, chooseAgain ? 2 : 1);
      },
    );
  }

  test("late source refresh cannot restore dismissed task feedback", () async {
    final fixture = _Fixture();
    await fixture.ready();
    await fixture.controller.scanDirectory("C:\\Moved");
    fixture.movePublishedRoot();
    final refresh = fixture.catalog.pendingLoad = Completer<LibrarySnapshot>();
    fixture.scanner.streams.single.add(
      const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
    );
    await fixture.scanner.streams.single.close();
    await flushRetainedScanMicrotasks();
    fixture.controller.dismissTaskFeedback();
    refresh.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
    await flushRetainedScanMicrotasks();
    expect(fixture.state.primaryScanSnapshot.hasFeedback, isFalse);
    expect(fixture.state.roots.last.path, "C:\\Moved");
    expect(fixture.state.status, LibraryStatus.completed);
  });

  test(
    "source refresh failure stays visible and releases retry admission",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.scanDirectory("C:\\Moved");
      fixture.movePublishedRoot();
      fixture.catalog.loadFailure = StateError("source refresh unavailable");
      fixture.scanner.streams.single.add(
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      expect(fixture.state.status, LibraryStatus.cancelled);
      expect(
        fixture.state.errorMessage,
        contains("source refresh unavailable"),
      );
      fixture.catalog.loadFailure = null;
      await fixture.controller.retry();
      expect(fixture.scanner.startedRoots, ["C:\\Moved", "C:\\Moved"]);
    },
  );

  test(
    "disposal retires pending source feedback without a late write",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.scanDirectory("C:\\Moved");
      fixture.movePublishedRoot();
      final refresh = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      fixture.scanner.streams.single.add(
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      fixture.dispose();
      refresh.complete(fixture.catalog.snapshot(const LibraryGalleryQuery()));
      await flushRetainedScanMicrotasks();
      expect(fixture.scanner.startedRoots, hasLength(1));
    },
  );

  test(
    "cancellation before Started still reloads a committed source location",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      fixture.catalog.roots[1] = const LibraryRoot(
        id: "other",
        path: "C:\\Moved",
        displayPath: "C:\\Moved",
        createdUnixMs: 1,
        assetCount: 6,
        issueCount: 0,
        activeScanId: "published",
        availability: LibraryRootAvailability.available,
      );
      fixture.scanner.streams.single.add(
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      expect(fixture.state.roots.last.path, "C:\\Moved");
      expect(fixture.state.status, LibraryStatus.cancelled);
      await fixture.controller.retry();
      expect(fixture.scanner.relocations, hasLength(1));
      expect(fixture.scanner.startedRoots, ["C:\\Moved"]);
    },
  );

  test(
    "cancelling the picker leaves the source and execution admission intact",
    () async {
      final fixture = _Fixture();
      fixture.picker.selection = null;
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      expect(fixture.scanner.relocations, isEmpty);
      expect(fixture.state.roots.last.path, otherPublishedRoot.path);
      expect(fixture.state.isBusy, isFalse);
      fixture.picker.selection = "C:\\Moved";
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      expect(fixture.scanner.relocations, hasLength(1));
    },
  );

  test(
    "pre-admission failure retries the selected root rather than adding another",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      final first = fixture.scanner.relocations.single;
      expect(first.root, same(otherPublishedRoot));
      expect(fixture.state.taskKind, LibraryTaskKind.update);
      fixture.scanner.streams.single.add(
        const LibraryScanFailed(
          code: "catalog_root_relocation_stale",
          message: "Source changed",
        ),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      await fixture.controller.retry();
      expect(fixture.scanner.relocations, hasLength(2));
      expect(fixture.scanner.relocations.last.root.id, otherPublishedRoot.id);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  test(
    "relocation conflict cannot redirect Retry to the occupying root",
    () async {
      final fixture = _Fixture(
        roots: const [
          LibraryRoot(
            id: "occupying",
            path: "C:\\Occupied",
            displayPath: "C:\\Occupied",
            activeScanId: "occupying-publication",
            createdUnixMs: 1,
            assetCount: 3,
            issueCount: 0,
            availability: LibraryRootAvailability.available,
          ),
          otherPublishedRoot,
        ],
      );
      fixture.picker.selection = "C:\\Occupied";
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      fixture.scanner.streams.single.add(
        const LibraryScanFailed(
          code: "catalog_root_relocation_conflict",
          message: "Destination belongs to another root",
        ),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      await fixture.controller.retry();
      expect(fixture.scanner.relocations, hasLength(2));
      expect(fixture.scanner.relocations.last.root.id, otherPublishedRoot.id);
      expect(fixture.scanner.startedRoots, isEmpty);
      expect(fixture.state.roots.last.path, otherPublishedRoot.path);
    },
  );

  test(
    "admitted relocation refreshes the source and cancelled retry uses its new path",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      await fixture.controller.chooseDirectoryAndRelocate(otherPublishedRoot);
      final call = fixture.scanner.relocations.single;
      fixture.catalog.roots[1] = const LibraryRoot(
        id: "other",
        path: "C:\\Moved",
        displayPath: "C:\\Moved",
        createdUnixMs: 1,
        assetCount: 6,
        issueCount: 0,
        activeScanId: "published",
        availability: LibraryRootAvailability.available,
      );
      fixture.scanner.streams.single.add(
        LibraryScanStarted(scanId: call.scanId, rootPath: call.path),
      );
      await flushRetainedScanMicrotasks();
      expect(fixture.state.roots.last.path, "C:\\Moved");
      expect(fixture.state.status, LibraryStatus.scanning);
      await fixture.controller.cancelScan();
      expect(fixture.scanner.cancelActiveIds, [call.scanId]);
      fixture.scanner.streams.single.add(
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      await fixture.scanner.streams.single.close();
      await flushRetainedScanMicrotasks();
      await fixture.controller.retry();
      expect(fixture.scanner.relocations, hasLength(1));
      expect(fixture.scanner.startedRoots, ["C:\\Moved"]);
    },
  );

  test(
    "late picker selection after disposal never changes the source",
    () async {
      final fixture = _Fixture();
      await fixture.ready();
      final selection = fixture.picker.pending = Completer<String?>();
      final choosing = fixture.controller.chooseDirectoryAndRelocate(
        otherPublishedRoot,
      );
      fixture.dispose();
      selection.complete("C:\\Moved");
      await choosing;
      expect(fixture.scanner.relocations, isEmpty);
    },
  );
}

class _Fixture {
  _Fixture({List<LibraryRoot>? roots}) {
    scanner.checkpoint = null;
    catalog = RetainedScanCatalog(scanner);
    if (roots != null) {
      catalog.roots
        ..clear()
        ..addAll(roots);
    }
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

  Future<void> ready() async {
    controller;
    await flushRetainedScanMicrotasks();
  }

  void movePublishedRoot() {
    catalog.roots[1] = const LibraryRoot(
      id: "other",
      path: "C:\\Moved",
      displayPath: "C:\\Moved",
      createdUnixMs: 1,
      assetCount: 6,
      issueCount: 0,
      activeScanId: "published",
      availability: LibraryRootAvailability.available,
    );
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
  String? selection = "C:\\Moved";
  Completer<String?>? pending;

  @override
  Future<String?> pickDirectory() async {
    calls += 1;
    return pending == null ? selection : pending!.future;
  }
}

class _Scanner extends RetainedScanScanner
    implements LibraryRootRelocationScanner {
  final relocations = <({LibraryRoot root, String scanId, String path})>[];
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
  Stream<LibraryScanUpdate> relocate({
    required LibraryRoot root,
    required String scanId,
    required String rootPath,
    required int previewEdge,
  }) {
    relocations.add((root: root, scanId: scanId, path: rootPath));
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
