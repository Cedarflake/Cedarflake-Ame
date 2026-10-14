import "dart:async";
import "dart:convert";
import "dart:io";
import "dart:typed_data";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_source_scope.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "pending image indicator stays at the viewport center during zoom and pan",
    (tester) async {
      tester.view.physicalSize = const Size(1000, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fixture = _Fixture();
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: LibraryViewerSourceScope(
              scheduler: fixture.scheduler,
              bufferLoader: fixture.loadBuffer,
              child: Scaffold(
                body: LibraryImageViewer(
                  asset: _asset("first", width: 4000, height: 2000),
                  openBehavior: ImageViewerOpenBehavior.actualSize,
                  onBack: () {},
                  onInformation: () {},
                  onCopyPath: () {},
                  onRevealFile: () {},
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      final viewport = find.byKey(
        const Key("library-image-interactive-viewer"),
      );
      final indicator = find.descendant(
        of: find.byKey(const Key("viewer-image-loading")),
        matching: find.byType(CircularProgressIndicator),
      );
      final center = tester.getRect(viewport).center;
      final bounds = tester.getRect(indicator);
      expect(bounds.center, center);
      await tester.tap(find.byKey(const Key("viewer-zoom-in")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.text("125%"), findsOneWidget);
      expect(tester.getRect(indicator), bounds);
      final transform = tester
          .widget<InteractiveViewer>(viewport)
          .transformationController!;
      final beforePan = transform.value.getTranslation().clone();
      await tester.drag(viewport, const Offset(180, 100));
      await tester.pump(kDoubleTapMinTime);
      expect(transform.value.getTranslation().x, isNot(beforePan.x));
      expect(tester.getRect(indicator), bounds);
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.reader.leases.single.closed, 0);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await _waitForImage(tester);
      expect(indicator, findsNothing);
      expect(fixture.reader.leases.single.closed, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  for (final hasPreview in [false, true]) {
    testWidgets(
      "original retry starts immediately after zoom and pan with preview $hasPreview",
      (tester) async {
        tester.view.physicalSize = const Size(1000, 700);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final fixture = _Fixture();
        var asset = _asset("first", width: 4000, height: 2000);
        if (hasPreview) {
          final directory = Directory.systemTemp.createTempSync(
            "ame-viewer-fallback-",
          );
          addTearDown(() => directory.deleteSync(recursive: true));
          final preview = File(
            "${directory.path}${Platform.pathSeparator}preview.png",
          )..writeAsBytesSync(_fixtureImageBytes());
          asset = asset.withPreview(
            previewPath: preview.path,
            width: asset.width,
            height: asset.height,
            previewStatus: LibraryPreviewStatus.ready,
          );
        }
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: LibraryViewerSourceScope(
                scheduler: fixture.scheduler,
                bufferLoader: fixture.loadBuffer,
                child: Scaffold(
                  body: LibraryImageViewer(
                    asset: asset,
                    openBehavior: ImageViewerOpenBehavior.actualSize,
                    onBack: () {},
                    onInformation: () {},
                    onCopyPath: () {},
                    onRevealFile: () {},
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pump();
        final viewport = find.byKey(
          const Key("library-image-interactive-viewer"),
        );
        final bounds = tester.getRect(viewport);
        await tester.tap(find.byKey(const Key("viewer-zoom-in")));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 200));
        final transform = tester
            .widget<InteractiveViewer>(viewport)
            .transformationController!;
        final beforePan = transform.value.getTranslation().clone();
        await tester.drag(viewport, const Offset(180, 100));
        await tester.pump(kDoubleTapMinTime);
        expect(transform.value.getTranslation().x, isNot(beforePan.x));
        final beforeFailure = transform.value.clone();

        fixture.buffers["approved-first"]!.completeError(
          StateError("fixture copy failed"),
        );
        await tester.pump();
        final retry = find.text("重试");
        final retryBounds = tester.getRect(retry);
        expect(bounds.contains(retryBounds.topLeft), isTrue);
        expect(bounds.contains(retryBounds.bottomRight), isTrue);
        expect(
          find.text(hasPreview ? "原图暂不可用，当前显示缩略图" : "无法打开原图"),
          findsOneWidget,
        );
        expect(transform.value, beforeFailure);
        expect(fixture.reader.leases.single.closed, 1);

        await tester.tap(find.byKey(const Key("viewer-zoom-in")));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 200));
        expect(tester.getRect(retry), retryBounds);
        final retryTransform = transform.value.clone();
        await tester.tap(retry);
        await tester.pump();
        expect(fixture.reader.opened, ["first", "first"]);
        expect(find.byKey(const Key("viewer-image-loading")), findsOneWidget);
        expect(transform.value, retryTransform);
        await tester.runAsync(() => fixture.finishBuffer("approved-first"));
        await _waitForImage(tester);
        expect(retry, findsNothing);
        expect(fixture.reader.leases.map((lease) => lease.closed), [1, 1]);
        expect(transform.value, retryTransform);
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets("inherited source dependencies keep an unchanged read alive", (
    tester,
  ) async {
    final fixture = _Fixture();
    await tester.pumpWidget(fixture.scopedViewer("first"));
    await tester.pump();
    await tester.pumpWidget(fixture.scopedViewer("first"));
    expect(fixture.reader.opened, ["first"]);
    expect(fixture.buffers.keys, ["approved-first"]);
    expect(fixture.reader.leases.single.closed, 0);
    await tester.runAsync(() => fixture.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(fixture.reader.leases.single.closed, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets("explicit source dependencies take precedence over the scope", (
    tester,
  ) async {
    final inherited = _Fixture();
    final explicit = _Fixture();
    await tester.pumpWidget(
      inherited.scopedViewer("first", overrides: explicit),
    );
    await tester.pump();
    expect(inherited.reader.opened, isEmpty);
    expect(inherited.buffers, isEmpty);
    expect(explicit.reader.opened, ["first"]);
    await tester.runAsync(() => explicit.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(explicit.reader.leases.single.closed, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets("a changed inherited scheduler resolves the same source afresh", (
    tester,
  ) async {
    final fixture = _Fixture();
    final replacement = _Fixture();
    await tester.pumpWidget(fixture.scopedViewer("first"));
    await tester.runAsync(() => fixture.finishBuffer("approved-first"));
    await _waitForImage(tester);
    await tester.pumpWidget(
      fixture.scopedViewer("first", inheritedScheduler: replacement.scheduler),
    );
    await tester.pump();
    expect(fixture.reader.opened, ["first"]);
    expect(fixture.reader.leases.single.closed, 1);
    expect(replacement.reader.opened, ["first"]);
    expect(replacement.reader.leases.single.closed, 0);
    await tester.runAsync(() => fixture.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(replacement.reader.leases.single.closed, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets("scope replacement retires a pending copy without early close", (
    tester,
  ) async {
    final fixture = _Fixture();
    final replacement = _Fixture();
    await tester.pumpWidget(fixture.scopedViewer("first"));
    await tester.pump();
    final retired = _sourceCompleter(tester);
    await tester.pumpWidget(
      fixture.scopedViewer("first", loader: replacement.loadBuffer),
    );
    await _pumpRetirement(tester);
    expect(retired.keepAlive, throwsStateError);
    expect(fixture.reader.opened, ["first"]);
    expect(fixture.reader.leases.single.closed, 0);
    final oldBuffer = await tester.runAsync(
      () => fixture.finishBuffer("approved-first"),
    );
    await tester.pump();
    expect(oldBuffer!.debugDisposed, isTrue);
    expect(fixture.reader.leases.first.closed, 1);
    expect(fixture.reader.opened, ["first", "first"]);
    expect(replacement.buffers.keys, ["approved-first"]);
    await tester.runAsync(() => replacement.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(fixture.reader.leases.last.closed, 1);
    expect(find.text("无法打开原图"), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "removing the scope restores defaults and preserves explicit admission",
    (tester) async {
      final fixture = _Fixture()..reader.gate = Completer();
      final key = GlobalKey();
      Widget viewer() => LibraryViewerImage(
        key: key,
        asset: _asset("first"),
        sourceReadScheduler: fixture.scheduler,
      );
      await tester.pumpWidget(
        MaterialApp(
          home: LibraryViewerSourceScope(
            scheduler: fixture.scheduler,
            bufferLoader: fixture.loadBuffer,
            child: viewer(),
          ),
        ),
      );
      await tester.pump();
      expect(
        _sourceProvider(tester),
        LibrarySourceImage(
          _asset("first"),
          scheduler: fixture.scheduler,
          bufferLoader: fixture.loadBuffer,
        ),
      );
      await tester.pumpWidget(MaterialApp(home: viewer()));
      expect(
        _sourceProvider(tester),
        LibrarySourceImage(_asset("first"), scheduler: fixture.scheduler),
      );
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.buffers, isEmpty);
      await tester.pumpWidget(const SizedBox.shrink());
      fixture.reader.gate!.complete();
      await tester.pump();
      expect(fixture.reader.leases.single.closed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("buffer reads only the approved path and waits for admission", (
    tester,
  ) async {
    final fixture = _Fixture()..reader.gate = Completer();
    await tester.pumpWidget(fixture.viewer("first"));
    await tester.pump();
    expect(fixture.reader.opened, ["first"]);
    expect(fixture.buffers, isEmpty);
    fixture.reader.gate!.complete();
    await tester.pump();
    expect(fixture.buffers.keys, ["approved-first"]);
    await tester.runAsync(() => fixture.finishBuffer("approved-first"));
    await _waitForImage(tester);
    expect(fixture.reader.leases.single.closed, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "fast navigation retains the active copy and opens only the latest selection",
    (tester) async {
      final fixture = _Fixture();
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      expect(fixture.buffers.keys, ["approved-first"]);
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(fixture.viewer("skipped"));
      await tester.pumpWidget(fixture.viewer("latest"));
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.reader.leases.first.closed, 0);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first", "latest"]);
      expect(fixture.buffers.keys, ["approved-first", "approved-latest"]);
      expect(fixture.reader.leases.first.closed, 1);
      await tester.runAsync(() => fixture.finishBuffer("approved-latest"));
      await _waitForImage(tester);
      expect(find.text("无法打开原图"), findsNothing);
      expect(fixture.reader.leases.last.closed, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "closing during a buffer copy releases only after copy completion",
    (tester) async {
      final fixture = _Fixture();
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.leases.single.closed, 0);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.leases.single.closed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "closing while admission is pending closes its late lease without opening a buffer",
    (tester) async {
      final fixture = _Fixture()..reader.gate = Completer();
      await tester.pumpWidget(fixture.viewer("first"));
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      fixture.reader.gate!.complete();
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.buffers, isEmpty);
      expect(fixture.reader.leases.single.closed, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a new selection survives admission completing after old stream retirement",
    (tester) async {
      final fixture = _Fixture()..reader.gate = Completer();
      await tester.pumpWidget(fixture.viewer("first"));
      final retired = _sourceCompleter(tester);
      await tester.pumpWidget(fixture.viewer("latest"));
      await _pumpRetirement(tester);
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first"]);
      expect(fixture.buffers, isEmpty);
      fixture.reader.gate!.complete();
      await tester.pump();
      expect(retired.keepAlive, throwsStateError);
      expect(fixture.reader.opened, ["first", "latest"]);
      expect(fixture.reader.leases.first.closed, 1);
      expect(fixture.buffers.keys, ["approved-latest"]);
      await tester.runAsync(() => fixture.finishBuffer("approved-latest"));
      await _waitForImage(tester);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "admission failure is visible and retry performs a fresh guarded read",
    (tester) async {
      final fixture = _Fixture()
        ..reader.failure = const LibrarySourceReadFailure(
          "viewer_source_unavailable",
          "Fixture is unavailable",
        );
      await tester.pumpWidget(fixture.viewer("first"));
      await tester.pump();
      expect(find.text("无法打开原图"), findsOneWidget);
      expect(fixture.buffers, isEmpty);
      fixture.reader.failure = null;
      await tester.tap(find.text("重试"));
      await tester.pump();
      expect(fixture.reader.opened, ["first", "first"]);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await tester.runAsync(() => fixture.finishBuffer("approved-first"));
      await _waitForImage(tester);
      expect(find.text("无法打开原图"), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("copy failure releases the source before showing retry", (
    tester,
  ) async {
    final fixture = _Fixture();
    await tester.pumpWidget(fixture.viewer("first"));
    await tester.pump();
    fixture.buffers["approved-first"]!.completeError(
      StateError("fixture copy failed"),
    );
    await tester.pump();
    expect(fixture.reader.leases.single.closed, 1);
    expect(find.text("无法打开原图"), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });
}

class _Fixture {
  final _Reader reader = _Reader();
  late final LibrarySourceReadScheduler scheduler = LibrarySourceReadScheduler(
    reader: reader,
  );
  final Map<String, Completer<ui.ImmutableBuffer>> buffers = {};

  Widget viewer(String id) => MaterialApp(
    home: Scaffold(
      body: LibraryViewerImage(
        asset: _asset(id),
        sourceReadScheduler: scheduler,
        sourceBufferLoader: loadBuffer,
      ),
    ),
  );

  Widget scopedViewer(
    String id, {
    _Fixture? overrides,
    LibrarySourceBufferLoader? loader,
    LibrarySourceReadScheduler? inheritedScheduler,
  }) => MaterialApp(
    home: LibraryViewerSourceScope(
      scheduler: inheritedScheduler ?? scheduler,
      bufferLoader: loader ?? loadBuffer,
      child: Scaffold(
        body: LibraryViewerImage(
          asset: _asset(id),
          sourceReadScheduler: overrides?.scheduler,
          sourceBufferLoader: overrides?.loadBuffer,
        ),
      ),
    ),
  );

  Future<ui.ImmutableBuffer> loadBuffer(String path) {
    final completion = Completer<ui.ImmutableBuffer>();
    buffers[path] = completion;
    return completion.future;
  }

  Future<ui.ImmutableBuffer> finishBuffer(String path) async {
    final buffer = await ui.ImmutableBuffer.fromUint8List(_fixtureImageBytes());
    buffers[path]!.complete(buffer);
    return buffer;
  }
}

Uint8List _fixtureImageBytes() => base64Decode(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
);

class _Reader implements LibrarySourceReader {
  final List<String> opened = [];
  final List<_Lease> leases = [];
  Completer<void>? gate;
  LibrarySourceReadFailure? failure;

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) async {
    opened.add(asset.locationId);
    await gate?.future;
    final error = failure;
    if (error != null) {
      throw error;
    }
    final lease = _Lease("approved-${asset.locationId}");
    leases.add(lease);
    return lease;
  }
}

class _Lease implements LibrarySourceReadLease {
  _Lease(this.sourcePath);
  @override
  final String sourcePath;
  int closed = 0;

  @override
  Future<void> close() async {
    closed += 1;
  }
}

Future<void> _waitForImage(WidgetTester tester) async {
  for (var attempt = 0; attempt < 100; attempt += 1) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
    if (find.byKey(const Key("viewer-image-loading")).evaluate().isEmpty &&
        tester
            .widgetList<RawImage>(find.byType(RawImage))
            .any((image) => image.image != null)) {
      return;
    }
  }
  fail("The guarded viewer did not display its decoded source");
}

ImageStreamCompleter _sourceCompleter(WidgetTester tester) {
  final stream = _sourceProvider(tester).resolve(ImageConfiguration.empty);
  final completer = stream.completer;
  if (completer == null) {
    throw StateError("The source image stream has not been admitted");
  }
  return completer;
}

LibrarySourceImage _sourceProvider(WidgetTester tester) {
  final image = tester
      .widgetList<Image>(find.byType(Image))
      .singleWhere((image) => image.image is LibrarySourceImage);
  return image.image as LibrarySourceImage;
}

Future<void> _pumpRetirement(WidgetTester tester) async {
  for (var frame = 0; frame < 4; frame += 1) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

LibraryAsset _asset(String id, {int width = 1, int height = 1}) => LibraryAsset(
  assetId: id,
  locationId: id,
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unapproved-$id",
  displayPath: "$id.png",
  relativePath: "$id.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: width,
  height: height,
);
