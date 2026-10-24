import "dart:async";
import "dart:io";
import "dart:typed_data";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/library_viewer_source_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_source_notice.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "source notices take the shared top slot before organization feedback",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-viewer-priority-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final source = File(
        "${directory.path}${Platform.pathSeparator}source.png",
      );
      final bytes = await tester.runAsync(() => _png(Colors.blue));
      source.writeAsBytesSync(bytes!);
      await tester.pumpWidget(
        _viewer(_asset(source.path), operationNotice: const Text("已收藏")),
      );
      await _waitForFrame(tester, () => find.text("已收藏").evaluate().isNotEmpty);
      await tester.pumpWidget(
        _viewer(
          _asset(source.path),
          operationNotice: const Text("已收藏"),
          notice: LibraryViewerSourceNoticeBanner(
            notice: LibraryViewerSourceNotice.changed,
            onBack: () {},
          ),
        ),
      );
      expect(find.text("已收藏"), findsNothing);
      expect(find.text(LibraryStrings.viewerSourceChanged), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  test(
    "source cache identity follows revision and generation, not preview",
    () {
      final original = _asset("same.png");
      final key = LibrarySourceImage(original);
      expect(LibrarySourceImage(_asset("same.png")), key);
      expect(
        LibrarySourceImage(
          original.withPreview(
            previewPath: "new-preview.webp",
            width: 2,
            height: 2,
            previewStatus: LibraryPreviewStatus.ready,
          ),
        ),
        key,
      );
      expect(LibrarySourceImage(_asset("same.png", generation: 2)), isNot(key));
      expect(
        LibrarySourceImage(_asset("same.png", scanId: "scan-2")),
        isNot(key),
      );
      expect(
        LibrarySourceImage(_asset("same.png", revision: "revision-2")),
        isNot(key),
      );
      expect(LibrarySourceImage(_asset("renamed.png")), isNot(key));
      expect(LibrarySourceImage(original, retryGeneration: 1), isNot(key));
    },
  );

  testWidgets(
    "same path and size with restored time displays the new source generation",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-viewer-source-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final file = File("${directory.path}${Platform.pathSeparator}source.png");
      final red = await tester.runAsync(() => _png(Colors.red));
      final blue = await tester.runAsync(() => _png(Colors.blue));
      file.writeAsBytesSync(red!);
      final timestamp = file.lastModifiedSync();
      await tester.pumpWidget(_viewer(_asset(file.path)));
      final before = tester.widget<Image>(find.byType(Image)).image;
      await _waitForFrame(
        tester,
        () => find.byType(RawImage).evaluate().isNotEmpty,
      );
      expect(await _visiblePixel(tester), [244, 67, 54, 255]);

      file.writeAsBytesSync(blue!);
      file.setLastModifiedSync(timestamp);
      expect(file.lengthSync(), red.length);
      expect(file.lastModifiedSync(), timestamp);
      await tester.pumpWidget(_viewer(_asset(file.path, generation: 2)));
      final after = tester.widget<Image>(find.byType(Image)).image;
      expect(after, isNot(before));
      await _waitForFrame(
        tester,
        () => find.byType(RawImage).evaluate().isNotEmpty,
      );
      expect(await _visiblePixel(tester), [33, 150, 243, 255]);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("retry resolves a fresh source stream for the same lease", (
    tester,
  ) async {
    final directory = Directory.systemTemp.createTempSync("ame-viewer-retry-");
    addTearDown(() => directory.deleteSync(recursive: true));
    final file = File("${directory.path}${Platform.pathSeparator}source.png");
    file.writeAsBytesSync(const []);
    await tester.pumpWidget(_viewer(_asset(file.path)));
    final before = tester.widget<Image>(find.byType(Image)).image;
    await _waitForFrame(
      tester,
      () => find.text("无法打开原图").evaluate().isNotEmpty,
    );
    expect(find.text("无法打开原图"), findsOneWidget);

    final bytes = await tester.runAsync(() => _png(Colors.blue));
    file.writeAsBytesSync(bytes!);
    await tester.tap(find.byKey(const Key("viewer-original-retry")));
    await tester.pump();
    final after = tester.widget<Image>(find.byType(Image)).image;
    expect(after, isNot(before));
    await _waitForFrame(
      tester,
      () => find.byType(RawImage).evaluate().isNotEmpty,
    );
    expect(await _visiblePixel(tester), [33, 150, 243, 255]);
    await tester.pump();
    expect(find.text("无法打开原图"), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets("source arrival retains an evicted but visible preview", (
    tester,
  ) async {
    final directory = Directory.systemTemp.createTempSync(
      "ame-viewer-handoff-",
    );
    addTearDown(() => directory.deleteSync(recursive: true));
    final preview = File(
      "${directory.path}${Platform.pathSeparator}preview.png",
    );
    final red = await tester.runAsync(() => _png(Colors.red));
    preview.writeAsBytesSync(red!);
    final source = Completer<ui.ImmutableBuffer>();
    final asset = _asset("pending-source.png").withPreview(
      previewPath: preview.path,
      width: 2,
      height: 2,
      previewStatus: LibraryPreviewStatus.ready,
    );
    await tester.pumpWidget(_viewer(asset, bufferLoader: (_) => source.future));
    final previewFinder = find.byWidgetPredicate(
      (widget) => widget is Image && widget.image == FileImage(preview),
    );
    await _waitForFrame(tester, () {
      final images = tester.widgetList<RawImage>(
        find.descendant(of: previewFinder, matching: find.byType(RawImage)),
      );
      return images.length == 1 && images.single.image != null;
    });
    final previewElement = tester.element(previewFinder);
    await FileImage(preview).evict();
    final blue = await tester.runAsync(() => _png(Colors.blue));
    final buffer = await tester.runAsync(
      () => ui.ImmutableBuffer.fromUint8List(blue!),
    );
    source.complete(buffer!);
    await _waitForFrame(
      tester,
      () => find.byKey(const Key("viewer-image-loading")).evaluate().isEmpty,
    );
    expect(tester.element(previewFinder), same(previewElement));
    expect(
      tester
          .widget<RawImage>(
            find.descendant(of: previewFinder, matching: find.byType(RawImage)),
          )
          .image,
      isNotNull,
    );
    expect(await _visiblePixel(tester), [33, 150, 243, 255]);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "confirmed removal replaces fallback retry without moving the image",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-viewer-notice-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final preview = File(
        "${directory.path}${Platform.pathSeparator}preview.png",
      );
      final bytes = await tester.runAsync(() => _png(Colors.blue));
      preview.writeAsBytesSync(bytes!);
      final asset =
          _asset(
            "${directory.path}${Platform.pathSeparator}missing.png",
          ).withPreview(
            previewPath: preview.path,
            width: 2,
            height: 2,
            previewStatus: LibraryPreviewStatus.ready,
          );
      await tester.pumpWidget(_viewer(asset));
      await _waitForFrame(
        tester,
        () => find
            .text(LibraryStrings.originalImagePreviewFallback)
            .evaluate()
            .isNotEmpty,
      );
      final imageRect = tester.getRect(find.byType(RawImage));
      var returned = false;
      await tester.pumpWidget(
        _viewer(
          asset,
          notice: LibraryViewerSourceNoticeBanner(
            notice: LibraryViewerSourceNotice.missing,
            onBack: () => returned = true,
          ),
        ),
      );
      expect(find.text(LibraryStrings.viewerSourceMissing), findsOneWidget);
      expect(find.byKey(const Key("viewer-original-retry")), findsNothing);
      expect(
        find.text(LibraryStrings.originalImagePreviewFallback),
        findsNothing,
      );
      expect(tester.getRect(find.byType(RawImage)), imageRect);
      expect(await _visiblePixel(tester), [33, 150, 243, 255]);
      await tester.tap(find.byKey(const Key("viewer-source-action")));
      expect(returned, isTrue);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  for (final corruptPreview in [false, true]) {
    testWidgets(
      "unreadable fallback does not claim to display a preview: corrupt $corruptPreview",
      (tester) async {
        final directory = Directory.systemTemp.createTempSync(
          "ame-viewer-unavailable-preview-",
        );
        addTearDown(() => directory.deleteSync(recursive: true));
        final source = File(
          "${directory.path}${Platform.pathSeparator}source.png",
        )..writeAsBytesSync(const []);
        final preview = File(
          "${directory.path}${Platform.pathSeparator}preview.png",
        );
        if (corruptPreview) {
          preview.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
        }
        final asset = _asset(source.path).withPreview(
          previewPath: preview.path,
          width: 2,
          height: 2,
          previewStatus: LibraryPreviewStatus.ready,
        );
        await tester.pumpWidget(_viewer(asset));
        await _waitForFrame(
          tester,
          () => find.text("无法打开原图").evaluate().isNotEmpty,
        );
        expect(
          find.text(LibraryStrings.originalImagePreviewFallback),
          findsNothing,
        );
        expect(find.text("重新打开"), findsOneWidget);

        final bytes = await tester.runAsync(() => _png(Colors.blue));
        source.writeAsBytesSync(bytes!);
        await tester.tap(find.byKey(const Key("viewer-original-retry")));
        await tester.pump();
        await _waitForFrame(
          tester,
          () =>
              find
                  .byKey(const Key("viewer-image-loading"))
                  .evaluate()
                  .isEmpty &&
              find.byType(RawImage).evaluate().isNotEmpty,
        );
        expect(await _visiblePixel(tester), [33, 150, 243, 255]);
        expect(find.text("无法打开原图"), findsNothing);
        expect(find.byKey(const Key("viewer-original-retry")), findsNothing);
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );
  }
}

final _sourceScheduler = LibrarySourceReadScheduler(
  reader: const _SourceReader(),
);

Widget _viewer(
  LibraryAsset asset, {
  LibrarySourceBufferLoader? bufferLoader,
  Widget? notice,
  Widget? operationNotice,
}) => MaterialApp(
  home: Scaffold(
    body: LibraryViewerImage(
      asset: asset,
      sourceReadScheduler: _sourceScheduler,
      sourceBufferLoader: bufferLoader,
      notice: notice,
      operationNotice: operationNotice,
    ),
  ),
);

class _SourceReader implements LibrarySourceReader {
  const _SourceReader();

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) async =>
      _SourceLease(asset.sourcePath);
}

class _SourceLease implements LibrarySourceReadLease {
  _SourceLease(this.sourcePath);

  @override
  final String sourcePath;

  @override
  Future<void> close() async {}
}

Future<Uint8List> _png(Color color) async {
  final recorder = ui.PictureRecorder();
  Canvas(
    recorder,
  ).drawRect(const Rect.fromLTWH(0, 0, 2, 2), Paint()..color = color);
  final picture = recorder.endRecording();
  final image = await picture.toImage(2, 2);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  picture.dispose();
  final encoded = data!.buffer.asUint8List();
  return Uint8List(256)..setRange(0, encoded.length, encoded);
}

Future<void> _waitForFrame(WidgetTester tester, bool Function() ready) async {
  for (var attempt = 0; attempt < 250; attempt += 1) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 20)),
    );
    await tester.pump(const Duration(milliseconds: 20));
    if (ready()) {
      return;
    }
  }
  fail(
    "The viewer did not publish its source image or failure within the frame budget",
  );
}

Future<List<int>> _visiblePixel(WidgetTester tester) async {
  await _waitForFrame(tester, () {
    final images = tester.widgetList<RawImage>(find.byType(RawImage)).toList();
    return images.length == 1 &&
        images.single.image != null &&
        (images.single.opacity?.value ?? 1) == 1;
  });
  final image = tester.widget<RawImage>(find.byType(RawImage)).image;
  if (image == null) {
    throw StateError("The viewer has no decoded source image");
  }
  final data = await tester.runAsync(image.toByteData);
  return data!.buffer.asUint8List().take(4).toList();
}

LibraryAsset _asset(
  String path, {
  int generation = 1,
  String revision = "r1",
  String scanId = "scan-1",
}) => LibraryAsset(
  assetId: "asset-1",
  locationId: "location-1",
  rootId: "root-1",
  activeScanId: scanId,
  sourcePath: path,
  displayPath: path,
  relativePath: "source.png",
  previewPath: "",
  fileSize: BigInt.from(256),
  modifiedUnixMs: 1234,
  sourceRevision: LibrarySourceRevisionEvidence(
    scheme: "test",
    value: revision,
  ),
  sourceGeneration: BigInt.from(generation),
  width: 2,
  height: 2,
  previewStatus: LibraryPreviewStatus.pending,
);
