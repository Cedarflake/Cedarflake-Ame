import "../application/library_catalog.dart";
import "../domain/library_models.dart";

typedef LibraryViewerNeighbor = ({LibraryAsset asset, int ordinal});

/// Resolves a retained selection after filmstrip paging has retired its gallery page.
class LibraryViewerNavigationWindow {
  const LibraryViewerNavigationWindow(this._catalog);

  final LibraryCatalog _catalog;

  Future<LibraryViewerNeighbor?> neighbor({
    required LibraryAsset selection,
    required LibraryGalleryQuery query,
    required BigInt? minimumRevision,
    required int fallbackOrdinal,
    required int step,
  }) async {
    final result = await _catalog.loadQuerySnapshot(
      maxItems: 64,
      query: query,
      anchor: LibraryQueryAnchor(
        requestedLocationId: selection.locationId,
        assetId: selection.assetId,
        fallbackGlobalItemIndex: fallbackOrdinal,
      ),
    );
    final page = result.snapshot;
    if (page.revision != result.timeline.revision ||
        page.queryId != result.timeline.queryId ||
        (minimumRevision != null && page.revision < minimumRevision)) {
      throw const LibraryCatalogFailure(
        code: "viewer_query_snapshot_stale",
        message: "图库正在更新，请重试切换图片。",
      );
    }
    final index = page.assets.indexWhere(
      (asset) => asset.assetId == selection.assetId,
    );
    final target = index + step;
    if (index < 0 || target < 0 || target >= page.assets.length) return null;
    final start = page.queryAnchorResolution?.windowStartItemOffset ?? 0;
    return (asset: page.assets[target], ordinal: start + target);
  }
}
