import "dart:io";
import "dart:typed_data";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
      expect(await tester.runAsync(() => _visiblePixel(tester)), [
        244,
        67,
        54,
        255,
      ]);

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
      expect(await tester.runAsync(() => _visiblePixel(tester)), [
        33,
        150,
        243,
        255,
      ]);
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
    await tester.tap(find.text("重试"));
    await tester.pump();
    final after = tester.widget<Image>(find.byType(Image)).image;
    expect(after, isNot(before));
    await _waitForFrame(
      tester,
      () => find.byType(RawImage).evaluate().isNotEmpty,
    );
    expect(await tester.runAsync(() => _visiblePixel(tester)), [
      33,
      150,
      243,
      255,
    ]);
    await tester.pump();
    expect(find.text("无法打开原图"), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });
}

Widget _viewer(LibraryAsset asset) => MaterialApp(
  home: Scaffold(body: LibraryViewerImage(asset: asset)),
);

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
  final image = tester.widget<RawImage>(find.byType(RawImage)).image;
  if (image == null) {
    throw StateError("The viewer has no decoded source image");
  }
  final data = await image.toByteData();
  return data!.buffer.asUint8List().take(4).toList();
}

LibraryAsset _asset(
  String path, {
  int generation = 1,
  String revision = "r1",
}) => LibraryAsset(
  assetId: "asset-1",
  locationId: "location-1",
  rootId: "root-1",
  activeScanId: "scan-1",
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
