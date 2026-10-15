import "dart:io";
import "dart:ui" as ui;

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  late ui.Image firstPixels;
  late ui.Image replacementPixels;

  setUp(() async {
    firstPixels = await _pixels(16, 12);
    replacementPixels = await _pixels(32, 24);
  });

  tearDown(() {
    PaintingBinding.instance.imageCache.clear();
    PaintingBinding.instance.imageCache.clearLiveImages();
    firstPixels.dispose();
    replacementPixels.dispose();
  });

  testWidgets("filmstrip keeps its placeholder until pixels are visible", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final frames = await _cacheFrames(asset);
    await _mount(tester, asset, _PreviewController());
    final bounds = tester.getRect(find.byKey(const ValueKey("location")));
    expect(_rawImage(tester).image, isNull);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);

    frames.show(firstPixels);
    await tester.pump();
    expect(_rawImage(tester).image!.isCloneOf(firstPixels), isTrue);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 60));
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 60));
    expect(find.byIcon(Symbols.image_rounded), findsNothing);
    expect(_opacity(tester), 1);
    expect(tester.getRect(find.byKey(const ValueKey("location"))), bounds);
    expect(tester.takeException(), isNull);
  });

  testWidgets("filmstrip keeps decoded pixels during a preview upgrade", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final first = await _cacheFrames(asset);
    first.show(firstPixels);
    final controller = _PreviewController();
    await _mount(tester, asset, controller);
    expect(_opacity(tester), 1);

    final replacement = _asset("larger.jpg");
    final next = await _cacheFrames(replacement);
    controller.replace(replacement);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(_rawImage(tester).image?.isCloneOf(firstPixels), isTrue);
    expect(find.byIcon(Symbols.image_rounded), findsNothing);
    expect(_opacity(tester), 1);

    next.show(replacementPixels);
    await tester.pump();
    expect(_rawImage(tester).image!.isCloneOf(replacementPixels), isTrue);
    expect(_opacity(tester), 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets("a changed source cannot retain the previous source's pixels", (
    tester,
  ) async {
    final asset = _asset("first.jpg");
    final first = await _cacheFrames(asset);
    first.show(firstPixels);
    final controller = _PreviewController();
    await _mount(tester, asset, controller);

    final replacement = _asset("rewritten.jpg", generation: 2);
    final next = await _cacheFrames(replacement);
    controller.replace(replacement);
    await tester.pump();
    expect(_rawImage(tester).image, isNull);
    expect(find.byIcon(Symbols.image_rounded), findsOneWidget);
    next.show(replacementPixels);
    await tester.pumpAndSettle();
    expect(_rawImage(tester).image!.isCloneOf(replacementPixels), isTrue);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _mount(
  WidgetTester tester,
  LibraryAsset asset,
  _PreviewController controller,
) async {
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryControllerProvider.overrideWith(() => controller),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState(assets: [asset]),
        ),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 400,
              height: 104,
              child: LibraryViewerFilmstrip(
                selection: asset,
                onSelected: (_) {},
                loadPrevious: () async {},
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

RawImage _rawImage(WidgetTester tester) =>
    tester.widget<RawImage>(find.byType(RawImage));

double _opacity(WidgetTester tester) =>
    tester.widget<Image>(find.byType(Image)).opacity?.value ?? 1;

Future<_Frames> _cacheFrames(LibraryAsset asset) async {
  final image = ResizeImage(FileImage(File(asset.previewPath)), width: 128);
  final key = await image.obtainKey(ImageConfiguration.empty);
  final frames = _Frames();
  PaintingBinding.instance.imageCache.putIfAbsent(key, () => frames);
  return frames;
}

Future<ui.Image> _pixels(int width, int height) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawColor(Colors.blue, BlendMode.src);
  canvas.drawRect(
    Rect.fromLTWH(0, 0, width / 2, height / 2),
    Paint()..color = Colors.orange,
  );
  final picture = recorder.endRecording();
  final pixels = await picture.toImage(width, height);
  picture.dispose();
  return pixels;
}

class _Frames extends ImageStreamCompleter {
  void show(ui.Image image) => setImage(ImageInfo(image: image.clone()));
}

class _PreviewController extends LibraryController {
  void replace(LibraryAsset asset) => state = state.copyWith(assets: [asset]);
}

LibraryAsset _asset(String previewPath, {int generation = 1}) => LibraryAsset(
  assetId: "asset",
  locationId: "location",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unused/photo.jpg",
  displayPath: "photo.jpg",
  relativePath: "photo.jpg",
  previewPath: previewPath,
  fileSize: BigInt.from(2000),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.from(generation),
  width: 640,
  height: 480,
  previewStatus: LibraryPreviewStatus.ready,
);
