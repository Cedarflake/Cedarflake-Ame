import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: 10000,
    buckets: [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 7000,
        aspectRatioSum: 8400,
      ),
      LibraryTimeBucket(
        monthKey: "2025-01",
        itemCount: 3000,
        aspectRatioSum: 3600,
      ),
    ],
  );

  test("keeps the complete scroll range stable when the window moves", () {
    final first = LibraryVirtualGalleryGeometry.calculate(
      timeline: timeline,
      availableWidth: 1200,
      viewportExtent: 800,
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
      loadedContentExtent: 5000,
      windowStartItemOffset: 0,
      loadedItemCount: 160,
      queryId: "query-1",
    );
    final middle = LibraryVirtualGalleryGeometry.calculate(
      timeline: timeline,
      availableWidth: 1200,
      viewportExtent: 800,
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
      loadedContentExtent: 9000,
      windowStartItemOffset: 5000,
      loadedItemCount: 160,
      queryId: "query-1",
    );

    expect(first.totalContentExtent, middle.totalContentExtent);
    expect(first.leadingExtent, 0);
    expect(middle.leadingExtent, greaterThan(0));
    expect(middle.trailingExtent, greaterThan(0));
    expect(middle.scrollOffsetForValue(0.5), middle.maximumScrollExtent * 0.5);
    expect(
      middle.valueForScrollOffset(middle.maximumScrollExtent * 0.25),
      closeTo(0.25, 0.0001),
    );
  });

  test(
    "uses the loaded content directly when the complete result is loaded",
    () {
      final completeTimeline = LibraryTimeline(
        revision: BigInt.one,
        queryId: "query-2",
        totalItems: 20,
        buckets: [
          LibraryTimeBucket(
            monthKey: "2026-08",
            itemCount: 20,
            aspectRatioSum: 24,
          ),
        ],
      );
      final geometry = LibraryVirtualGalleryGeometry.calculate(
        timeline: completeTimeline,
        availableWidth: 1200,
        viewportExtent: 800,
        layoutShape: GalleryLayoutShape.equalHeight,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
        loadedContentExtent: 1400,
        windowStartItemOffset: 0,
        loadedItemCount: 20,
        queryId: "query-2",
      );

      expect(geometry.isVirtualized, isFalse);
      expect(geometry.totalContentExtent, 1400);
      expect(geometry.leadingExtent, 0);
      expect(geometry.trailingExtent, 0);
    },
  );
}
