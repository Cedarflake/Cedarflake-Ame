import "dart:async";
import "dart:io";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("buckets preview decode widths across small layout changes", () {
    expect(libraryPreviewDecodeWidth(40, 1), 128);
    expect(libraryPreviewDecodeWidth(127, 1), 128);
    expect(libraryPreviewDecodeWidth(129, 1), 256);
    expect(libraryPreviewDecodeWidth(180, 1), 256);
    expect(libraryPreviewDecodeWidth(181, 1), 256);
    expect(libraryPreviewDecodeWidth(129, 2), 512);
    expect(libraryPreviewDecodeWidth(600, 2), 512);
  });

  testWidgets(
    "repairs a ready preview after Flutter cannot decode it",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-preview-repair-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      final previewer = _RecordingPreviewer(asset);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 && previewer.requests.isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
      }
      await tester.pump();

      expect(previewer.requests, [(locationId: "location-ready", retry: true)]);
      expect(find.byKey(const Key("preview-retry-location-ready")), findsOne);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );
}

LibraryAsset _readyAsset(String previewPath) {
  return LibraryAsset(
    assetId: "asset-ready",
    locationId: "location-ready",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\ready.jpg",
    displayPath: "C:\\Pictures\\ready.jpg",
    relativePath: "ready.jpg",
    previewPath: previewPath,
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

class _RecordingPreviewer implements LibraryPreviewer {
  _RecordingPreviewer(this.asset);

  final LibraryAsset asset;
  final requests = <({String locationId, bool retry})>[];

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
  }) {
    requests.add((locationId: locationId, retry: retry));
    return Future.value(
      asset.withPreview(
        previewPath: asset.previewPath,
        width: asset.width,
        height: asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_repair_failed",
        previewIssueMessage: "Repair failed",
      ),
    );
  }
}

class _FakeCatalog implements LibraryCatalog {
  const _FakeCatalog(this.snapshot);

  final LibrarySnapshot snapshot;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length.toDouble(),
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _FakeScanner implements LibraryScanner {
  const _FakeScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }
}
