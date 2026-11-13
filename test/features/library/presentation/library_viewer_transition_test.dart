import "dart:async";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/adapters/platform/library_preview_image.dart";
import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/ports/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_chrome.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_card_overlays.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image_flight.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_launch.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_transition.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flutter/rendering.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets("card corners retain their screen radius under viewer zoom", (
    tester,
  ) async {
    const bounds = Rect.fromLTWH(100, 80, 120, 120);
    await tester.pumpWidget(
      MaterialApp(
        home: RepaintBoundary(
          key: const Key("rounded-flight"),
          child: ColoredBox(
            color: Colors.white,
            child: LibraryViewerFlightScope(
              origin: LibraryViewerOrigin(
                locationId: "photo",
                geometry: (bounds: bounds, visibleBounds: bounds),
                currentGeometry: () => (bounds: bounds, visibleBounds: bounds),
                fit: BoxFit.cover,
                radius: 20,
              ),
              animation: const AlwaysStoppedAnimation(0),
              child: Transform.scale(
                scale: 2,
                child: const LibraryViewerImageFlight(
                  locationId: "photo",
                  imageSize: Size(400, 200),
                  child: ColoredBox(color: Colors.blue),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    final boundary = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(const Key("rounded-flight")),
    );
    await tester.runAsync(() async {
      final capture = await boundary.toImage();
      try {
        final bytes = (await capture.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        int redAt(int x, int y) => bytes.getUint8((y * capture.width + x) * 4);
        expect(redAt(101, 81), 255);
        expect(redAt(110, 90), (Colors.blue.r * 255).round());
      } finally {
        capture.dispose();
      }
    });
  });

  testWidgets(
    "return clips the travelling image before reaching a partial card",
    (tester) async {
      final scroll = ScrollController(initialScrollOffset: 140);
      addTearDown(scroll.dispose);
      final gallery = Stack(
        children: [
          Positioned(
            top: 120,
            left: 0,
            width: 320,
            height: 180,
            child: SingleChildScrollView(
              controller: scroll,
              child: SizedBox(height: 600, child: _launchCard()),
            ),
          ),
        ],
      );
      final image = find.byKey(const Key("flight-image"));
      await tester.pumpWidget(_host(gallery, null));
      await tester.tapAt(const Offset(160, 150));
      await tester.pumpWidget(_host(gallery, _flightViewer()));
      await tester.pumpAndSettle();
      await tester.pumpWidget(_host(gallery, null));
      for (
        var frame = 0;
        frame < 30 &&
            image.evaluate().isNotEmpty &&
            tester.getRect(image).width > 300;
        frame++
      ) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      expect(image, findsOneWidget);
      expect(tester.getRect(image).top, lessThan(120));
      final boundary = tester.renderObject<RenderRepaintBoundary>(
        find.byKey(const Key("flight-capture")),
      );
      await tester.runAsync(() async {
        final capture = await boundary.toImage();
        try {
          final bytes = (await capture.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          ))!;
          int? firstBlueRow;
          for (var y = 0; y < capture.height && firstBlueRow == null; y++) {
            for (var x = 0; x < capture.width; x++) {
              final offset = (y * capture.width + x) * 4;
              if (bytes.getUint8(offset) == (Colors.blue.r * 255).round() &&
                  bytes.getUint8(offset + 1) == (Colors.blue.g * 255).round() &&
                  bytes.getUint8(offset + 2) == (Colors.blue.b * 255).round()) {
                firstBlueRow = y;
                break;
              }
            }
          }
          expect(firstBlueRow, isNotNull);
          expect(firstBlueRow, greaterThanOrEqualTo(120));
        } finally {
          capture.dispose();
        }
      });
      await tester.pumpAndSettle();
      expect(image, findsNothing);
      expect(scroll.offset, 140);
    },
  );

  testWidgets("image travels from its card and back while chrome stays still", (
    tester,
  ) async {
    final gallery = Stack(
      children: [
        _launchCard(),
        const Positioned(
          left: 100,
          top: 80,
          child: LibraryViewerCardOverlays(child: Text("badge")),
        ),
      ],
    );
    final badgeFade = find
        .ancestor(of: find.text("badge"), matching: find.byType(FadeTransition))
        .first;
    double badgeOpacity() =>
        tester.widget<FadeTransition>(badgeFade).opacity.value;
    final image = find.byKey(const Key("flight-image"));
    await tester.pumpWidget(_host(gallery, null));
    expect(badgeOpacity(), 1);
    await tester.tap(find.byKey(const Key("launch-card-photo")));
    await tester.pumpWidget(_host(gallery, _flightViewer()));
    final initial = tester.getRect(image);
    expect(initial.center, const Offset(160, 140));
    expect(initial.size, const Size(240, 120));
    final flight = tester.renderObject<RenderLibraryViewerImageFlight>(
      find.byType(LibraryViewerImageFlight),
    );
    final clip = flight.debugLayer! as ClipRectLayer;
    final roundedClip = clip.firstChild! as ClipRRectLayer;
    final transform = roundedClip.firstChild! as TransformLayer;
    final chrome = tester.getRect(find.text("chrome"));
    await tester.pump(const Duration(milliseconds: 60));
    expect(flight.debugLayer, same(clip));
    expect(clip.firstChild, same(roundedClip));
    expect(roundedClip.firstChild, same(transform));
    final moving = tester.getRect(image);
    expect(moving.width, greaterThan(initial.width));
    expect(moving.width, lessThan(800));
    expect(moving.width / moving.height, closeTo(2, 0.0001));
    expect(moving.center.dx, greaterThan(initial.center.dx));
    expect(tester.getRect(find.text("chrome")), chrome);
    expect(badgeOpacity(), inExclusiveRange(0, 1));
    await tester.pumpAndSettle();
    final full = tester.getRect(image);
    expect(full.size, const Size(800, 400));
    expect(flight.debugLayer, isNull);

    await tester.pumpWidget(_host(gallery, null));
    await tester.pump(const Duration(milliseconds: 40));
    expect(badgeOpacity(), 0);
    expect(tester.getRect(image).width, lessThan(full.width));
    expect(tester.getRect(image).center.dx, lessThan(full.center.dx));
    expect(tester.getRect(find.text("chrome")), chrome);
    await _expectOpaqueImage(tester, image);
    for (var frame = 0; image.evaluate().isNotEmpty && frame < 120; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      expect(badgeOpacity(), 0);
    }
    expect(image, findsNothing);
    await tester.pump(const Duration(milliseconds: 50));
    expect(badgeOpacity(), inExclusiveRange(0, 1));
    await tester.pumpAndSettle();
    expect(badgeOpacity(), 1);
    expect(find.byKey(const Key("launch-card-photo")), findsOneWidget);
  });

  testWidgets("a changed image or retired card never flies to an old target", (
    tester,
  ) async {
    final gallery = _launchCard();
    final image = find.byKey(const Key("flight-image"));
    await tester.pumpWidget(_host(gallery, null));
    await tester.tap(find.byKey(const Key("launch-card-photo")));
    await tester.pumpWidget(_host(gallery, _flightViewer()));
    await tester.pumpAndSettle();
    final full = tester.getRect(image);
    await tester.pumpWidget(_host(gallery, _flightViewer(locationId: "other")));
    await tester.pumpWidget(_host(gallery, null));
    await tester.pump(const Duration(milliseconds: 40));
    expect(tester.getRect(image), full);
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key("launch-card-photo")));
    await tester.pumpWidget(_host(gallery, _flightViewer()));
    await tester.pumpAndSettle();
    await tester.pumpWidget(_host(const SizedBox(), _flightViewer()));
    await tester.pumpWidget(_host(const SizedBox(), null));
    await tester.pump(const Duration(milliseconds: 40));
    expect(tester.getRect(image), full);
    await tester.pumpAndSettle();
    expect(image, findsNothing);
    expect(tester.takeException(), isNull);

    final adjacent = Stack(
      children: [
        gallery,
        _launchCard(locationId: "other", left: 600),
      ],
    );
    await tester.pumpWidget(_host(adjacent, null));
    await tester.tap(find.byKey(const Key("launch-card-photo")));
    await tester.pumpWidget(_host(adjacent, _flightViewer()));
    await tester.pumpAndSettle();
    await tester.pumpWidget(
      _host(adjacent, _flightViewer(locationId: "other")),
    );
    await tester.pumpWidget(_host(adjacent, null));
    await tester.pump(const Duration(milliseconds: 40));
    expect(tester.getRect(image).width, lessThan(full.width));
    expect(tester.getRect(image).center.dx, greaterThan(full.center.dx));
    await tester.pumpAndSettle();
  });

  testWidgets(
    "overlay semantics remain reachable through zero opacity and interrupted exit",
    (tester) async {
      final semantics = tester.ensureSemantics();
      final gallery = Column(
        children: [
          const Text("gallery"),
          Slider(value: 0.5, onChanged: (_) {}),
        ],
      );
      final viewer = Column(
        children: [
          const Text("viewer"),
          Slider(value: 0.5, onChanged: (_) {}),
        ],
      );
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer first transparent frame",
      );
      await tester.pump(const Duration(milliseconds: 200));
      await tester.pumpWidget(_host(gallery, null));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer exit input retired",
      );
      await tester.pump(const Duration(milliseconds: 30));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpAndSettle();
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "viewer retired",
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      semantics.dispose();
    },
  );
  testWidgets(
    "return restores gallery position and input before the fade finishes",
    (tester) async {
      final scroll = ScrollController(initialScrollOffset: 200);
      addTearDown(scroll.dispose);
      var galleryTaps = 0;
      var viewerTaps = 0;
      final gallery = ListView(
        controller: scroll,
        children: [
          const SizedBox(height: 200),
          TextButton(
            key: const Key("gallery-action"),
            onPressed: () => galleryTaps++,
            child: const Text("gallery"),
          ),
          const SizedBox(height: 1000),
        ],
      );
      final viewer = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => viewerTaps++,
        child: const Center(child: Text("viewer")),
      );
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      expect(scroll.offset, 200);
      await tester.pumpWidget(_host(gallery, null));
      expect(find.text("viewer"), findsOneWidget);
      expect(
        LibraryViewerActivity.isActiveOf(tester.element(find.text("viewer"))),
        isFalse,
      );
      await tester.tap(find.byKey(const Key("gallery-action")));
      expect(galleryTaps, 1);
      expect(viewerTaps, 0);
      expect(scroll.offset, 200);
      await tester.pumpAndSettle();
      expect(find.text("viewer"), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    "quick reopen keeps one active viewer and ignores an older exit",
    (tester) async {
      const gallery = Text("gallery");
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, const Text("first")));
      await tester.pump(const Duration(milliseconds: 100));
      await tester.pumpWidget(_host(gallery, null));
      await tester.pump(const Duration(milliseconds: 20));
      await tester.pumpWidget(_host(gallery, const Text("second")));
      expect(find.text("first"), findsNothing);
      expect(find.text("second"), findsOneWidget);
      expect(
        LibraryViewerActivity.isActiveOf(tester.element(find.text("second"))),
        isTrue,
      );
      await tester.pumpAndSettle();
      expect(find.text("second"), findsOneWidget);
      expect(
        tester
            .widget<FadeTransition>(find.byType(FadeTransition).last)
            .opacity
            .value,
        1,
      );
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("reduced motion removes the closing viewer in the same frame", (
    tester,
  ) async {
    final gallery = _launchCard();
    await tester.pumpWidget(_host(gallery, null, reducedMotion: true));
    await tester.tap(find.byKey(const Key("launch-card-photo")));
    await tester.pumpWidget(
      _host(gallery, _flightViewer(), reducedMotion: true),
    );
    expect(
      tester.getRect(find.byKey(const Key("flight-image"))).size,
      const Size(800, 400),
    );
    await tester.pumpWidget(_host(gallery, null, reducedMotion: true));
    expect(find.byKey(const Key("flight-image")), findsNothing);
    expect(find.byKey(const Key("launch-card-photo")), findsOneWidget);
    expect(tester.binding.transientCallbackCount, 0);
  });

  testWidgets(
    "closing retires a pending source before the visual exit completes",
    (tester) async {
      final reader = _DelayedReader();
      final scheduler = LibrarySourceReadScheduler(reader: reader);
      var bufferReads = 0;
      final viewer = LibraryViewerImage(
        previewImages: createLibraryPreviewImage,
        asset: _asset(),
        sourceReadScheduler: scheduler,
        sourceBufferLoader: (_) async {
          bufferReads++;
          throw StateError("A retired source must not be decoded");
        },
      );
      const gallery = Text("gallery");
      await tester.pumpWidget(_host(gallery, null));
      await tester.pumpWidget(_host(gallery, viewer));
      await tester.pump(const Duration(milliseconds: 200));
      expect(reader.requests, 1);
      await tester.pumpWidget(_host(gallery, null));
      expect(find.byType(LibraryViewerImage), findsOneWidget);
      final lease = _Lease();
      reader.result.complete(lease);
      await tester.pump();
      expect(lease.closes, 1);
      expect(bufferReads, 0);
      await tester.pumpAndSettle();
      expect(find.byType(LibraryViewerImage), findsNothing);
      expect(tester.takeException(), isNull);
      PaintingBinding.instance.imageCache.clear();
      PaintingBinding.instance.imageCache.clearLiveImages();
    },
  );
}

Widget _host(Widget gallery, Widget? viewer, {bool reducedMotion = false}) =>
    MaterialApp(
      home: MediaQuery(
        data: MediaQueryData(disableAnimations: reducedMotion),
        child: RepaintBoundary(
          key: const Key("flight-capture"),
          child: Scaffold(
            body: LibraryViewerTransition(gallery: gallery, viewer: viewer),
          ),
        ),
      ),
    );

Widget _launchCard({String locationId = "photo", double left = 100}) => Align(
  alignment: Alignment.topLeft,
  child: Padding(
    padding: EdgeInsets.only(left: left, top: 80),
    child: LibraryViewerLaunch(
      locationId: locationId,
      onOpen: () {},
      builder: (context, anchor, open) => GestureDetector(
        onTap: open,
        child: SizedBox.square(
          key: anchor,
          dimension: 120,
          child: ColoredBox(
            color: Colors.blue,
            key: Key("launch-card-$locationId"),
          ),
        ),
      ),
    ),
  ),
);

Widget _flightViewer({String locationId = "photo"}) => Column(
  children: [
    const LibraryViewerChrome(
      child: SizedBox(height: 40, child: Text("chrome")),
    ),
    Expanded(
      child: LibraryViewerImageFlight(
        locationId: locationId,
        imageSize: const Size(400, 200),
        child: const FittedBox(
          child: SizedBox(
            width: 400,
            height: 200,
            child: ColoredBox(color: Colors.blue, key: Key("flight-image")),
          ),
        ),
      ),
    ),
  ],
);

Future<void> _expectOpaqueImage(WidgetTester tester, Finder image) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(const Key("flight-capture")),
  );
  final center = boundary.globalToLocal(tester.getCenter(image));
  await tester.runAsync(() async {
    final capture = await boundary.toImage();
    try {
      final bytes = (await capture.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final offset =
          (center.dy.floor() * capture.width + center.dx.floor()) * 4;
      expect(bytes.getUint8(offset), (Colors.blue.r * 255).round());
      expect(bytes.getUint8(offset + 1), (Colors.blue.g * 255).round());
      expect(bytes.getUint8(offset + 2), (Colors.blue.b * 255).round());
    } finally {
      capture.dispose();
    }
  });
}

LibraryAsset _asset() => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unopened-source.png",
  displayPath: "unopened-source.png",
  relativePath: "unopened-source.png",
  previewPath: "",
  fileSize: BigInt.from(256),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 200,
  height: 100,
  previewStatus: LibraryPreviewStatus.pending,
);

class _DelayedReader implements LibrarySourceReader {
  final result = Completer<LibrarySourceReadLease>();
  var requests = 0;

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) {
    requests++;
    return result.future;
  }
}

class _Lease implements LibrarySourceReadLease {
  var closes = 0;

  @override
  String get sourcePath => "unopened-source.png";

  @override
  Future<void> close() async {
    closes++;
  }
}
