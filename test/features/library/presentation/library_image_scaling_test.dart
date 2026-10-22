import "dart:io";
import "dart:typed_data";
import "dart:ui" as ui;

import "package:cedarflake_ame/app/presentation/ame_scroll_motion.dart";
import "package:cedarflake_ame/features/library/application/library_source_read_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_cover_preview_size.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/gallery_wall_fixture.dart";

void main() {
  test("cover demand includes the cropped extent and stays bounded", () {
    for (final density in [1.0, 1.5, 2.0]) {
      final portrait = libraryCoverPreviewSize(
        imageWidth: 1200,
        imageHeight: 3600,
        width: 128,
        height: 138,
        devicePixelRatio: density,
      );
      expect(portrait.previewEdge / 3, greaterThanOrEqualTo(128 * density));
      final panorama = libraryCoverPreviewSize(
        imageWidth: 3600,
        imageHeight: 1200,
        width: 138,
        height: 128,
        devicePixelRatio: density,
      );
      expect(panorama.decodeWidth / 3, greaterThanOrEqualTo(128 * density));
    }
    final extreme = libraryCoverPreviewSize(
      imageWidth: 1,
      imageHeight: 100000,
      width: 200,
      height: 200,
      devicePixelRatio: 3,
    );
    expect(extreme.previewEdge, 1024);
    expect(extreme.decodeWidth, 1024);
  });

  testWidgets("viewer averages fine detail when reducing an original", (
    tester,
  ) async {
    final bytes = await tester.runAsync(
      () => _png(1024, 1024, (canvas) {
        canvas.drawColor(Colors.white, BlendMode.src);
        for (var x = 0; x < 1024; x += 7) {
          canvas.drawRect(
            Rect.fromLTWH(x.toDouble(), 0, 3, 1024),
            Paint()..color = Colors.black,
          );
        }
      }),
    );
    final boundary = GlobalKey();
    await tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: RepaintBoundary(
            key: boundary,
            child: SizedBox.square(
              dimension: 128,
              child: LibraryViewerImage(
                asset: galleryAsset(0, "").withPreview(
                  previewPath: "",
                  width: 1024,
                  height: 1024,
                  previewStatus: LibraryPreviewStatus.pending,
                ),
                sourceReadScheduler: LibrarySourceReadScheduler(
                  reader: const _SourceReader(),
                ),
                sourceBufferLoader: (_) =>
                    ui.ImmutableBuffer.fromUint8List(bytes!),
              ),
            ),
          ),
        ),
      ),
    );
    await _waitForImage(tester);
    final pixels = await _capture(tester, boundary);
    final samples = [for (var x = 8; x < 120; x++) pixels.at(x, 64).first];
    expect(samples, everyElement(inInclusiveRange(125, 165)));
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final withManifest in [false, true]) {
    for (final shape in GalleryLayoutShape.values) {
      testWidgets(
        "narrow image requests sharper pixels and keeps its crop ($withManifest, $shape)",
        (tester) async {
          final directory = Directory.systemTemp.createTempSync(
            "ame-image-fit-",
          );
          addTearDown(() => directory.deleteSync(recursive: true));
          final file = File("${directory.path}/portrait.png");
          final bytes = await tester.runAsync(
            () => _png(128, 1024, (canvas) {
              canvas.drawColor(const Color(0xff00ff00), BlendMode.src);
              canvas.drawRect(
                const Rect.fromLTWH(0, 0, 128, 192),
                Paint()..color = const Color(0xffff0000),
              );
              canvas.drawRect(
                const Rect.fromLTWH(0, 832, 128, 192),
                Paint()..color = const Color(0xff0000ff),
              );
            }),
          );
          file.writeAsBytesSync(bytes!);
          final asset = galleryAsset(0, "").withPreview(
            previewPath: file.path,
            width: 128,
            height: 1024,
            previewStatus: LibraryPreviewStatus.ready,
          );
          final state = galleryBoundaryState(
            itemCount: 1,
            windowStart: 0,
            windowEnd: 1,
            hasPrevious: false,
            hasNext: false,
          ).copyWith(assets: [asset]);
          final controller = RecordingGalleryController();
          final scroll = AmeScrollController();
          addTearDown(scroll.dispose);
          final boundary = GlobalKey();
          final opened = <LibraryAsset>[];
          await tester.pumpWidget(
            ProviderScope(
              child: MaterialApp(
                home: RepaintBoundary(
                  key: boundary,
                  child: LibraryGalleryWall(
                    state: state,
                    controller: controller,
                    scrollController: scroll,
                    layoutShape: shape,
                    thumbnailSize: GalleryThumbnailSize.medium,
                    selection: GallerySelection.empty(state.queryId),
                    isSelecting: false,
                    layoutManifest: withManifest ? galleryManifest(1) : null,
                    onOpen: opened.add,
                    onToggleSelection: (_) {},
                    onViewInformation: (_) {},
                    onCopyPath: (_) {},
                    onRevealFile: (_) {},
                    onVisiblePositionChanged: (_) {},
                    onLoadPrevious: controller.loadPreviousPage,
                    onLayoutChanged: (_, _) {},
                  ),
                ),
              ),
            ),
          );
          await _waitForImage(tester);
          final rect = tester.getRect(find.byType(LibraryPhotoTile));
          final pixels = await _capture(tester, boundary);
          final top = pixels.at(
            rect.center.dx.round(),
            (rect.top + rect.height * .1).round(),
          );
          final bottom = pixels.at(
            rect.center.dx.round(),
            (rect.bottom - rect.height * .1).round(),
          );
          expect(top, [0, 255, 0]);
          expect(bottom, [0, 255, 0]);
          expect(
            controller.previewDemandRequests.last.edges[asset.locationId],
            1024,
          );
          expect(
            tester.widget<RawImage>(find.byType(RawImage)).image!.width,
            128,
          );
          expect(rect.width, greaterThanOrEqualTo(128));
          await tester.tapAt(Offset(rect.left + 12, rect.center.dy));
          expect(opened.single.locationId, asset.locationId);
          await tester.pumpWidget(const SizedBox.shrink());
          expect(tester.takeException(), isNull);
        },
      );
    }
  }
}

Future<Uint8List> _png(
  int width,
  int height,
  void Function(Canvas) paint,
) async {
  final recorder = ui.PictureRecorder();
  paint(Canvas(recorder));
  final picture = recorder.endRecording();
  final image = await picture.toImage(width, height);
  final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  picture.dispose();
  return bytes!.buffer.asUint8List();
}

Future<void> _waitForImage(WidgetTester tester) async {
  for (var attempt = 0; attempt < 250; attempt++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 20)),
    );
    await tester.pump(const Duration(milliseconds: 20));
    final images = tester.widgetList<RawImage>(find.byType(RawImage)).toList();
    if (images.length == 1 &&
        images.single.image != null &&
        (images.single.opacity?.value ?? 1) == 1) {
      return;
    }
  }
  fail(
    "The image did not finish decoding and revealing within the frame budget",
  );
}

Future<_Pixels> _capture(WidgetTester tester, GlobalKey key) async {
  final boundary =
      key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  return (await tester.runAsync(() async {
    final image = await boundary.toImage();
    final data = await image.toByteData();
    final pixels = _Pixels(image.width, data!.buffer.asUint8List());
    image.dispose();
    return pixels;
  }))!;
}

class _Pixels {
  _Pixels(this.width, this.bytes);
  final int width;
  final Uint8List bytes;

  List<int> at(int x, int y) {
    final offset = (y * width + x) * 4;
    return bytes.sublist(offset, offset + 3);
  }
}

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
