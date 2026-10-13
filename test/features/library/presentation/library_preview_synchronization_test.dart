import "dart:async";
import "dart:io";
import "dart:typed_data";
import "dart:ui" as ui;

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  for (final obsoleteResultFirst in [true, false]) {
    testWidgets("stationary gallery loads the new source after supersession, "
        "obsolete result first: $obsoleteResultFirst", (tester) async {
      final fixture = await _GalleryFixture.mount(tester);
      final originalOffset = fixture.scrollPosition(tester).pixels;
      final current = _asset(2);

      if (obsoleteResultFirst) {
        fixture.previewer.calls.single.completion.completeError(
          const LibraryPreviewFailure(
            code: "preview_request_superseded",
            message: "Source changed after decoding",
          ),
        );
        await _pumpFrames(tester, 4);
        expect(fixture.previewer.calls, hasLength(1));
        expect(
          find.byKey(const Key("library-preview-pending")),
          findsOneWidget,
        );
      }

      fixture.publish([current], revision: 2);
      await _pumpUntil(
        tester,
        () => fixture.libraryState(tester).catalogRevision == BigInt.two,
      );
      expect(
        fixture.libraryState(tester).assets.single.sourceGeneration,
        BigInt.two,
      );
      expect(fixture.catalog.queryReads, 1);
      if (!obsoleteResultFirst) {
        expect(fixture.previewer.calls, hasLength(1));
        fixture.previewer.calls.single.completion.completeError(
          const LibraryPreviewFailure(
            code: "preview_request_superseded",
            message: "Source changed after decoding",
          ),
        );
      }
      await _pumpUntil(tester, () => fixture.previewer.calls.length == 2);

      final requests = fixture.previewer.calls;
      expect(requests.map((call) => call.generation), [BigInt.one, BigInt.two]);
      expect(requests.map((call) => call.force), [false, false]);
      expect(requests.last.revision, current.sourceRevision);
      requests.last.completion.complete(fixture.ready(current));
      await _pumpUntil(tester, () => _decodedImage(tester) != null);
      final pixels = await tester.runAsync(
        () => _decodedImage(tester)!.toByteData(),
      );
      expect(pixels!.buffer.asUint8List().take(4), [18, 104, 212, 255]);
      await _pumpFrames(tester, 12);

      expect(fixture.previewer.calls, hasLength(2));
      expect(fixture.catalog.queryReads, 1);
      expect(fixture.scrollPosition(tester).pixels, originalOffset);
      expect(find.byKey(const Key("library-preview-pending")), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets("authoritative removal retires a late ready preview", (
    tester,
  ) async {
    final fixture = await _GalleryFixture.mount(tester);
    final obsoleteRequest = fixture.previewer.calls.single;
    fixture.publish(const [], revision: 2);
    await _pumpUntil(
      tester,
      () => find.byType(LibraryPhotoTile).evaluate().isEmpty,
    );
    expect(fixture.catalog.queryReads, 1);

    obsoleteRequest.completion.complete(fixture.ready(_asset(1)));
    await _pumpFrames(tester, 12);
    expect(find.byType(LibraryPhotoTile), findsNothing);
    expect(_decodedImage(tester), isNull);
    expect(fixture.previewer.calls, hasLength(1));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    expect(container.read(libraryControllerProvider).assets, isEmpty);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });
}

class _GalleryFixture {
  _GalleryFixture(this.previewFile);

  final File previewFile;
  final _GalleryCatalog catalog = _GalleryCatalog();
  final _Previewer previewer = _Previewer();
  final _Synchronization synchronization = _Synchronization();

  static Future<_GalleryFixture> mount(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final directory = Directory.systemTemp.createTempSync("ame-preview-sync-");
    final file = File("${directory.path}${Platform.pathSeparator}current.png");
    addTearDown(() {
      if (file.existsSync()) {
        file.deleteSync();
      }
      directory.deleteSync();
    });
    final bytes = await tester.runAsync(_previewBytes);
    file.writeAsBytesSync(bytes!);
    final fixture = _GalleryFixture(file);
    final scanner = RetainedScanScanner()..checkpoint = null;
    addTearDown(scanner.dispose);
    addTearDown(fixture.synchronization.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(
              fixture.catalog.snapshot,
            ).copyWith(timeline: fixture.catalog.timeline),
          ),
          libraryCatalogProvider.overrideWithValue(fixture.catalog),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryPreviewerProvider.overrideWithValue(fixture.previewer),
          librarySynchronizationProvider.overrideWithValue(
            fixture.synchronization,
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await _pumpUntil(tester, () => fixture.previewer.calls.isNotEmpty);
    expect(fixture.previewer.calls, hasLength(1));
    expect(fixture.catalog.queryReads, 0);
    expect(find.byType(LibraryPhotoTile), findsOneWidget);
    return fixture;
  }

  void publish(List<LibraryAsset> assets, {required int revision}) {
    catalog.assets = assets;
    catalog.revision = BigInt.from(revision);
    synchronization.publish(catalog.revision);
  }

  LibraryAsset ready(LibraryAsset asset) => asset.withPreview(
    previewPath: previewFile.path,
    width: 32,
    height: 24,
    previewStatus: LibraryPreviewStatus.ready,
  );

  LibraryState libraryState(WidgetTester tester) => ProviderScope.containerOf(
    tester.element(find.byType(AmeApp)),
  ).read(libraryControllerProvider);

  ScrollPosition scrollPosition(WidgetTester tester) => tester
      .state<ScrollableState>(
        find.descendant(
          of: find.byKey(const Key("library-photo-wall")),
          matching: find.byType(Scrollable),
        ),
      )
      .position;
}

class _GalleryCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  List<LibraryAsset> assets = [_asset(1)];
  BigInt revision = BigInt.one;
  int queryReads = 0;

  LibrarySnapshot get snapshot => LibrarySnapshot(
    catalogPath: "controlled.sqlite3",
    queryId: "source-replacement",
    revision: revision,
    assets: assets,
    roots: [
      LibraryRoot(
        id: "other",
        path: r"C:\Controlled",
        displayPath: r"C:\Controlled",
        activeScanId: "published",
        createdUnixMs: 1,
        assetCount: assets.length,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
    ],
  );

  LibraryTimeline get timeline => LibraryTimeline(
    revision: revision,
    queryId: snapshot.queryId,
    totalItems: assets.length,
    buckets: [
      if (assets.isNotEmpty)
        LibraryTimeBucket(itemCount: assets.length, aspectRatioSum: 4 / 3),
    ],
  );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    queryReads++;
    return snapshot;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw StateError("No time navigation is admitted in this fixture");

  @override
  Future<bool> unregisterRoot(String rootId) =>
      throw StateError("No root-removal command is admitted in this fixture");
}

class _Synchronization extends InertLibrarySynchronization {
  final _updates = StreamController<LibrarySynchronizationSnapshot>.broadcast();

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void publish(BigInt revision) {
    _updates.add(
      LibrarySynchronizationSnapshot(
        isRunning: true,
        catalogRevision: revision,
        appliedMutationCount: 1,
        roots: const {},
      ),
    );
  }

  @override
  Future<void> dispose() => _updates.close();
}

class _Previewer implements LibraryPreviewer {
  final calls =
      <
        ({
          BigInt generation,
          LibrarySourceRevisionEvidence? revision,
          bool force,
          Completer<LibraryAsset> completion,
        })
      >[];

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    expect(locationId, "location");
    expect(expectedRootId, "other");
    expect(expectedScanId, "published");
    final completion = Completer<LibraryAsset>();
    calls.add((
      generation: expectedSourceGeneration,
      revision: expectedSourceRevision,
      force: force,
      completion: completion,
    ));
    return completion.future;
  }
}

LibraryAsset _asset(int generation) => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "other",
  activeScanId: "published",
  sourcePath: r"C:\Controlled\source.png",
  displayPath: r"C:\Controlled\source.png",
  relativePath: "source.png",
  previewPath: "",
  fileSize: BigInt.from(256),
  modifiedUnixMs: 1234,
  sourceRevision: LibrarySourceRevisionEvidence(
    scheme: "test",
    value: "$generation",
  ),
  sourceGeneration: BigInt.from(generation),
  width: 32,
  height: 24,
  previewStatus: LibraryPreviewStatus.pending,
);

Future<Uint8List> _previewBytes() async {
  final recorder = ui.PictureRecorder();
  Canvas(recorder).drawRect(
    const Rect.fromLTWH(0, 0, 32, 24),
    Paint()..color = const Color(0xFF1268D4),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(32, 24);
  try {
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    return bytes!.buffer.asUint8List();
  } finally {
    image.dispose();
    picture.dispose();
  }
}

ui.Image? _decodedImage(WidgetTester tester) {
  final images = find.descendant(
    of: find.byType(LibraryPhotoTile),
    matching: find.byType(RawImage),
  );
  return images.evaluate().isEmpty
      ? null
      : tester.widget<RawImage>(images).image;
}

Future<void> _pumpFrames(WidgetTester tester, int count) async {
  for (var frame = 0; frame < count; frame++) {
    await tester.pump(const Duration(milliseconds: 16));
    expect(find.text(LibraryStrings.retryPreview), findsNothing);
    expect(tester.takeException(), isNull);
  }
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() ready) async {
  for (var frame = 0; frame < 250; frame++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 20)),
    );
    await _pumpFrames(tester, 1);
    if (ready()) {
      return;
    }
  }
  fail(
    "The stationary gallery did not deliver the expected preview transition",
  );
}
