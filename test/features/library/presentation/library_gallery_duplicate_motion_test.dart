import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/application_test_bindings.dart";
import "support/gallery_wall_fixture.dart";

void main() {
  for (final shape in GalleryLayoutShape.values) {
    testWidgets("background merge moves cards continuously in $shape", (
      tester,
    ) async {
      final state = ValueNotifier(
        _publication(List.generate(100, (i) => i), 1),
      );
      final scroll = ScrollController();
      final controller = RecordingGalleryController();
      addTearDown(state.dispose);
      addTearDown(scroll.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: applicationTestOverrides(),
          child: MaterialApp(
            home: Material(
              child: ValueListenableBuilder(
                valueListenable: state,
                builder: (context, value, _) => LibraryGalleryWall(
                  state: value,
                  controller: controller,
                  scrollController: scroll,
                  layoutShape: shape,
                  thumbnailSize: GalleryThumbnailSize.medium,
                  selection: GallerySelection.empty(value.queryId),
                  isSelecting: false,
                  layoutManifest: value.layoutManifest,
                  onOpen: (_) {},
                  onToggleSelection: (_) {},
                  onViewInformation: (_) {},
                  onCopyPath: (_) {},
                  onRevealFile: (_) {},
                  onVisiblePositionChanged: (_) {},
                  onLoadPrevious: () async {},
                  onLayoutChanged: (_, _) {},
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final retained = find.byKey(const ValueKey("location-2"));
      final frames = [tester.getRect(retained)];
      state.value = _publication([
        for (var i = 0; i < 100; i++)
          if (i != 1) i,
      ], 2);
      await tester.pump();
      expect(tester.getRect(retained), rectMoreOrLessEquals(frames.first));
      for (var frame = 0; frame < 45; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        frames.add(tester.getRect(retained));
        expect(scroll.offset, 0);
      }
      expect(frames.last, isNot(frames.first));
      for (var frame = 1; frame < frames.length; frame++) {
        for (final edge in <double Function(Rect)>[
          (rect) => rect.left,
          (rect) => rect.top,
          (rect) => rect.right,
          (rect) => rect.bottom,
        ]) {
          final remaining = (edge(frames[frame]) - edge(frames.last)).abs();
          final prior = (edge(frames[frame - 1]) - edge(frames.last)).abs();
          expect(
            remaining,
            lessThanOrEqualTo(prior + 0.001),
            reason: "frame $frame",
          );
        }
      }
      final settled = tester.getRect(retained);
      state.value = _publication([
        for (var i = 0; i < 100; i++)
          if (i != 1 && i != 90) i,
      ], 3);
      for (var frame = 0; frame < 45; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        expect(
          tester.getRect(retained),
          rectMoreOrLessEquals(settled),
          reason: "offscreen merge frame $frame",
        );
        expect(scroll.offset, 0);
      }
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }
}

LibraryState _publication(List<int> ids, int version) {
  final revision = BigInt.from(version);
  const queryId = "duplicate-motion";
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: queryId,
    totalItems: ids.length,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: queryId,
      totalItems: ids.length,
      startOrdinal: 0,
      locationIds: [for (final id in ids) "location-$id"],
      aspectRatioMilli: Uint16List.fromList([
        for (final id in ids) 1000 + id % 3 * 200,
      ]),
      dateGroupIndices: Uint16List(ids.length),
      dateGroups: const ["2026-08-10"],
      flags: Uint8List.fromList(
        List.filled(ids.length, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return galleryBoundaryState(
    itemCount: 100,
    windowStart: 0,
    windowEnd: 100,
    hasPrevious: false,
    hasNext: false,
  ).copyWith(
    catalogRevision: revision,
    queryId: queryId,
    query: const LibraryGalleryQuery(
      duplicateMode: LibraryGalleryDuplicateMode.merged,
    ),
    assets: [for (final id in ids) galleryAsset(id, "preview unavailable")],
    layoutManifest: builder.build(),
    timeline: LibraryTimeline(
      revision: revision,
      queryId: queryId,
      totalItems: ids.length,
      totalSourceItems: 100,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: ids.length,
          aspectRatioSum: ids.length.toDouble(),
        ),
      ],
    ),
  );
}
