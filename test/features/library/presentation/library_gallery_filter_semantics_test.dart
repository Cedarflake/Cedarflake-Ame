import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);
  testWidgets(
    "gallery filter preserves semantics through asynchronous replacement",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1280, 800));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final semantics = tester.ensureSemantics();
      try {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              libraryControllerProvider.overrideWith(_FilterLibrary.new),
              libraryGalleryLayoutManifestProvider.overrideWith((
                ref,
                request,
              ) async {
                await Future<void>.delayed(const Duration(milliseconds: 60));
                return _manifest(_state(request.query));
              }),
            ],
            child: const AmeApp(),
          ),
        );
        await tester.pumpAndSettle();
        for (final selection in ["全部图片", "合并重复图片", "全部图片"]) {
          await tester.tap(find.byKey(const Key("library-filter-menu")));
          await tester.pumpAndSettle();
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "filter-open-$selection",
          );
          expect(find.byIcon(Symbols.circle_rounded), findsNWidgets(2));
          final longestLabel = find.text("合并重复图片");
          final paragraph = tester.renderObject<RenderParagraph>(longestLabel);
          final labelBoxes = paragraph.getBoxesForSelection(
            const TextSelection(baseOffset: 0, extentOffset: 6),
          );
          final labelRight =
              tester.getTopLeft(longestLabel).dx + labelBoxes.last.right;
          final menu = find
              .ancestor(of: longestLabel, matching: find.byType(Material))
              .first;
          expect(
            tester.getRect(menu).right - labelRight,
            closeTo(AmeMenuMetrics.horizontalPadding, 1),
          );
          expect(
            find.byWidgetPredicate((widget) => widget is CheckedPopupMenuItem),
            findsNothing,
          );
          await tester.tap(
            find.ancestor(
              of: find.text(selection),
              matching: find.byWidgetPredicate(
                (widget) => widget is PopupMenuItem,
              ),
            ),
          );
          for (var frame = 0; frame < 40; frame++) {
            await tester.pump(const Duration(milliseconds: 16));
            RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
              trace: "filter-replacement-$selection-$frame",
            );
          }
          expect(tester.takeException(), isNull);
        }
        await tester.pumpWidget(const SizedBox.shrink());
      } finally {
        semantics.dispose();
      }
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );
}

class _FilterLibrary extends ReviewFixtureLibrary {
  @override
  LibraryState build() => _state(
    const LibraryGalleryQuery(
      duplicateMode: LibraryGalleryDuplicateMode.merged,
    ),
  );

  @override
  Future<bool> updateQuery(
    LibraryGalleryQuery query, {
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
    bool forceRefresh = false,
    BigInt? minimumCatalogRevision,
    bool showRefreshingStatus = true,
  }) async {
    await Future<void>.delayed(const Duration(milliseconds: 50));
    state = _state(query);
    return true;
  }
}

LibraryState _state(LibraryGalleryQuery query) {
  final merged = query.duplicateMode == LibraryGalleryDuplicateMode.merged;
  final count = merged ? 72 : 288;
  final queryId = query.duplicateMode.name;
  return LibraryState(
    status: LibraryStatus.completed,
    query: query,
    queryId: queryId,
    catalogRevision: BigInt.one,
    roots: const [
      LibraryRoot(
        id: "root",
        path: "C:\\Fixture",
        displayPath: "C:\\Fixture",
        createdUnixMs: 1,
        assetCount: 288,
        issueCount: 0,
      ),
    ],
    assets: [
      for (var index = 0; index < count; index++)
        _asset(merged ? index * 4 : index),
    ],
    timeline: LibraryTimeline(
      revision: BigInt.one,
      queryId: queryId,
      totalItems: count,
      totalSourceItems: 288,
      buckets: [
        LibraryTimeBucket(
          monthKey: "1970-01",
          itemCount: count,
          aspectRatioSum: count * 1.5,
        ),
      ],
    ),
  );
}

LibraryGalleryLayoutManifest _manifest(LibraryState state) {
  final count = state.assets.length;
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: state.catalogRevision!,
    queryId: state.queryId,
    totalItems: count,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: state.catalogRevision!,
      queryId: state.queryId,
      totalItems: count,
      startOrdinal: 0,
      locationIds: [for (final asset in state.assets) asset.locationId],
      aspectRatioMilli: Uint16List.fromList([
        for (final asset in state.assets) asset.width * 1000 ~/ asset.height,
      ]),
      dateGroupIndices: Uint16List(count),
      dateGroups: const ["1970-01-01"],
      flags: Uint8List.fromList(
        List.filled(count, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}

LibraryAsset _asset(int index) => LibraryAsset(
  assetId: "asset-$index",
  locationId: "location-$index",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "C:\\Fixture\\$index.png",
  displayPath: "C:\\Fixture\\$index.png",
  relativePath: "$index.png",
  previewPath: "",
  previewStatus: LibraryPreviewStatus.pending,
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: index.isEven ? 640 : 1920,
  height: index.isEven ? 480 : 1080,
);
