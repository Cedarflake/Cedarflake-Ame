import "dart:async";
import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";

void main() {
  testWidgets(
    "a newer overlapping date target retires prepend after page publication before its frame",
    (tester) async {
      final fixture = await _mount(tester, delayedManifest: true);
      await _seek(tester, 0.65);
      await _frames(tester);
      fixture.catalog.holdPrevious = true;
      await _wheel(tester, -20);
      await _frames(tester);
      expect(fixture.catalog.pendingPrevious, isNotNull);
      fixture.catalog.completePrevious();
      await tester.idle();
      expect(fixture.state.isLoadingPreviousPage, isFalse);
      expect(fixture.state.windowStartItemOffset, 340);
      await _seek(tester, 0.69);
      await _frames(tester);
      expect(fixture.state.windowStartItemOffset, 744);
      expect(fixture.state.assets.first.locationId, "location-744");
      expect(_position(tester).pixels, lessThan(100));
      expect(find.byKey(const ValueKey("location-744")), findsOneWidget);
      await fixture.dispose(tester);
    },
  );

  testWidgets(
    "window-local prepend retains the visible tile after later wheel input",
    (tester) async {
      final fixture = await _mount(tester, delayedManifest: true);
      await _seek(tester, 0.65);
      await _frames(tester);
      final anchorId = fixture.state.assets.first.locationId;
      fixture.catalog.holdPrevious = true;
      await _wheel(tester, -20);
      await _frames(tester);
      expect(fixture.catalog.pendingPrevious, isNotNull);
      await _wheel(tester, -10);
      final anchor = find.byKey(ValueKey(anchorId));
      final before = tester.getTopLeft(anchor).dy;
      fixture.catalog.completePrevious();
      await _frames(tester);
      expect(find.byKey(ValueKey(anchorId)), findsOneWidget);
      expect(tester.getTopLeft(anchor).dy, closeTo(before, 1));
      expect(fixture.state.isLoadingPreviousPage, isFalse);
      await fixture.dispose(tester);
    },
  );

  testWidgets("previous detail publication keeps query-wide scroll geometry", (
    tester,
  ) async {
    final fixture = await _mount(tester);
    await _seek(tester, 0.45);
    await _frames(tester);
    final before = fixture.state;
    expect(before.windowStartItemOffset, greaterThan(500));
    expect(before.hasPreviousAssets, isTrue);
    expect(before.hasMoreAssets, isTrue);

    fixture.catalog.holdPrevious = true;
    await _wheel(tester, -80);
    expect(fixture.catalog.pendingPrevious, isNotNull);
    final pixels = _position(tester).pixels;
    final extent = _position(tester).maxScrollExtent;
    fixture.catalog.completePrevious();
    await _frames(tester);

    expect(
      fixture.state.windowStartItemOffset,
      lessThan(before.windowStartItemOffset),
    );
    expect(_position(tester).maxScrollExtent, closeTo(extent, 0.01));
    expect(_position(tester).pixels, closeTo(pixels, 1));
    expect(fixture.state.hasMoreAssets, isTrue);
    await _wheel(tester, 600);
    expect(_position(tester).pixels, greaterThan(pixels));
    await fixture.dispose(tester);
  });

  for (final delayedManifest in [false, true]) {
    testWidgets(
      "upward input during a held time read preserves downward reachability (manifest delayed=$delayedManifest)",
      (tester) async {
        final fixture = await _mount(tester, delayedManifest: delayedManifest);
        await _seek(tester, 0.65);
        await _frames(tester);
        final beforeSeek = _description(tester, fixture);
        fixture.catalog.holdTime = true;
        await _seek(tester, 0.35);
        expect(fixture.catalog.pendingTime, isNotNull);
        expect(fixture.state.isLoadingTimeAnchor, isTrue);
        await _wheel(tester, -160);
        final duringRead = _description(tester, fixture);
        expect(fixture.catalog.pendingTime!.isCompleted, isFalse);
        fixture.catalog.completeTime();
        if (delayedManifest) {
          fixture.manifest.complete(fixture.catalog.manifest);
        }
        await _frames(tester);
        final position = _position(tester);
        final pixels = position.pixels;
        expect(
          position.maxScrollExtent - pixels,
          greaterThan(3000),
          reason:
              "before=$beforeSeek during=$duringRead after=${_description(tester, fixture)}",
        );
        await _wheel(tester, 2600);
        await _frames(tester);
        expect(_position(tester).pixels, greaterThan(pixels + 1000));
        expect(fixture.state.isLoadingTimeAnchor, isFalse);
        expect(fixture.state.isLoadingPage, isFalse);
        expect(fixture.state.isLoadingPreviousPage, isFalse);
        expect(fixture.state.pageErrorMessage, isNull);
        expect(tester.takeException(), isNull);
        await fixture.dispose(tester);
      },
    );
  }
}

Future<_Fixture> _mount(
  WidgetTester tester, {
  bool delayedManifest = false,
}) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final fixture = _Fixture();
  if (!delayedManifest) {
    fixture.manifest.complete(fixture.catalog.manifest);
  }
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: fixture.container,
      child: const AmeApp(),
    ),
  );
  await _frames(tester);
  return fixture;
}

Future<void> _frames(WidgetTester tester) async {
  for (var frame = 0; frame < 10; frame += 1) {
    await tester.pump(const Duration(milliseconds: 25));
  }
}

Future<void> _seek(WidgetTester tester, double value) async {
  final slider = tester.widget<Slider>(
    find.byKey(const Key("timeline-slider")),
  );
  slider.onChangeStart?.call(slider.value);
  slider.onChanged?.call(value);
  slider.onChangeEnd?.call(value);
  await tester.pump();
  await tester.pump();
}

Future<void> _wheel(WidgetTester tester, double delta) async {
  await tester.sendEventToBinding(
    PointerScrollEvent(
      position: tester.getCenter(find.byKey(const Key("library-photo-wall"))),
      scrollDelta: Offset(0, delta),
    ),
  );
  await tester.pump();
  await tester.pump();
}

ScrollPosition _position(WidgetTester tester) => tester
    .state<ScrollableState>(
      find.descendant(
        of: find.byKey(const Key("library-photo-wall")),
        matching: find.byType(Scrollable),
      ),
    )
    .position;

String _description(WidgetTester tester, _Fixture fixture) {
  final position = _position(tester);
  final state = fixture.state;
  return "pixels=${position.pixels} max=${position.maxScrollExtent} "
      "start=${state.windowStartItemOffset} count=${state.assets.length} "
      "previous=${state.hasPreviousAssets} next=${state.hasMoreAssets} "
      "time=${state.isLoadingTimeAnchor} page=${state.isLoadingPage} "
      "rail=${tester.widget<Slider>(find.byKey(const Key('timeline-slider'))).value} "
      "reads=${fixture.catalog.reads}";
}

class _Fixture {
  _Fixture() {
    container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            catalog.page(0, 160),
          ).copyWith(timeline: catalog.timeline),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
        libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(manifest),
        libraryPreviewerProvider.overrideWithValue(_PendingPreviewer()),
      ],
    );
    addTearDown(container.dispose);
  }

  final catalog = _Catalog();
  final manifest = _ManifestLoader();
  late final ProviderContainer container;
  LibraryState get state => container.read(libraryControllerProvider);

  Future<void> dispose(WidgetTester tester) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(seconds: 1));
  }
}

class _Catalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  static const count = 2400;
  static const root = LibraryRoot(
    id: "root-1",
    path: "C:\\Generated",
    displayPath: "C:\\Generated",
    activeScanId: "scan-1",
    createdUnixMs: 1,
    assetCount: count,
    issueCount: 0,
    availability: LibraryRootAvailability.available,
  );
  final assets = [
    for (var index = 0; index < count; index += 1)
      LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: root.id,
        activeScanId: "scan-1",
        sourcePath: "C:\\Generated\\$index.png",
        displayPath: "C:\\Generated\\$index.png",
        relativePath: "$index.png",
        previewPath: "",
        fileSize: BigInt.one,
        modifiedUnixMs: 1,
        sourceRevision: null,
        sourceGeneration: BigInt.one,
        width: 128,
        height: 128,
        captureTime: LibraryCaptureTimeEvidence(
          localTime: "${2026 - index ~/ 100}-09-01T12:00:00.000000000",
          source: LibraryCaptureTimeSource.exifDateTimeOriginal,
          rawValue: "${2026 - index ~/ 100}:09:01 12:00:00",
        ),
      ),
  ];
  late final timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: count,
    buckets: [
      for (var year = 2026; year > 2002; year -= 1)
        LibraryTimeBucket(
          monthKey: "$year-09",
          itemCount: 100,
          aspectRatioSum: 100,
        ),
    ],
  );
  bool holdTime = false;
  bool holdPrevious = false;
  Completer<LibrarySnapshot>? pendingTime;
  Completer<LibrarySnapshot>? pendingPrevious;
  LibrarySnapshot? _timeResult;
  LibrarySnapshot? _previousResult;
  final reads = <String>[];

  LibrarySnapshot page(int start, int length) {
    final end = (start + length).clamp(0, count);
    return LibrarySnapshot(
      catalogPath: "C:\\GeneratedCatalog\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [root],
      assets: assets.sublist(start, end),
      previousCursor: start > 0 ? _cursor(start) : null,
      nextCursor: end < count ? _cursor(end - 1) : null,
    );
  }

  LibraryCatalogCursor _cursor(int index) => LibraryCatalogCursor(
    revision: BigInt.one,
    queryId: "query-1",
    primaryMissing: false,
    primaryText: "",
    primaryNumber: index,
    rootId: root.id,
    locationId: "location-$index",
  );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    reads.add(
      "page before=${before?.primaryNumber} after=${after?.primaryNumber}",
    );
    if (before != null) {
      final end = before.primaryNumber;
      final start = (end - maxItems).clamp(0, count);
      final result = page(start, end - start);
      if (holdPrevious) {
        holdPrevious = false;
        _previousResult = result;
        pendingPrevious = Completer();
        return pendingPrevious!.future;
      }
      return result;
    }
    return page(after == null ? 0 : after.primaryNumber + 1, maxItems);
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    final year = int.parse(anchor.monthKey!.substring(0, 4));
    reads.add("time year=$year offset=${anchor.itemOffset}");
    final result = page((2026 - year) * 100 + anchor.itemOffset, maxItems);
    if (holdTime) {
      holdTime = false;
      _timeResult = result;
      pendingTime = Completer();
      return pendingTime!.future;
    }
    return result;
  }

  void completeTime() => pendingTime!.complete(_timeResult!);
  void completePrevious() => pendingPrevious!.complete(_previousResult!);

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<bool> unregisterRoot(String rootId) async => false;

  LibraryGalleryLayoutManifest get manifest {
    final builder = LibraryGalleryLayoutManifestBuilder(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: count,
    );
    builder.append(
      LibraryGalleryLayoutManifestChunk(
        revision: BigInt.one,
        queryId: "query-1",
        totalItems: count,
        startOrdinal: 0,
        locationIds: [for (final asset in assets) asset.locationId],
        aspectRatioMilli: Uint16List.fromList(List.filled(count, 1000)),
        dateGroupIndices: Uint16List.fromList([
          for (var i = 0; i < count; i += 1) i ~/ 100,
        ]),
        dateGroups: [
          for (var year = 2026; year > 2002; year -= 1) "$year-09-01",
        ],
        flags: Uint8List.fromList(
          List.filled(count, libraryGalleryLayoutDimensionsKnownFlag),
        ),
      ),
    );
    return builder.build();
  }
}

class _ManifestLoader implements LibraryGalleryLayoutManifestLoader {
  final _result = Completer<LibraryGalleryLayoutManifest>();
  void complete(LibraryGalleryLayoutManifest value) => _result.complete(value);
  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) => _result.future;
}

class _PendingPreviewer implements LibraryPreviewer {
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
  }) => Completer<LibraryAsset>().future;
}
