import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";

mixin LibraryQuerySnapshotFixture implements LibraryCatalog {
  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    final LibrarySnapshot snapshot;
    if (anchor != null &&
        anchor.assetId != null &&
        this is LibraryStableQueryAnchorCatalog) {
      snapshot = await (this as LibraryStableQueryAnchorCatalog)
          .loadAroundAsset(
            maxItems: maxItems,
            query: query,
            requestedLocationId: anchor.requestedLocationId,
            anchorAssetId: anchor.assetId!,
            fallbackGlobalItemIndex: anchor.fallbackGlobalItemIndex,
          );
    } else if (anchor != null && this is LibraryQueryAnchorCatalog) {
      snapshot = await (this as LibraryQueryAnchorCatalog).loadAroundLocation(
        maxItems: maxItems,
        query: query,
        anchorLocationId: anchor.requestedLocationId,
      );
    } else {
      snapshot = await load(maxItems: maxItems, query: query);
    }
    return LibraryQuerySnapshot(
      snapshot: snapshot,
      timeline: await loadTimeline(query),
    );
  }
}
