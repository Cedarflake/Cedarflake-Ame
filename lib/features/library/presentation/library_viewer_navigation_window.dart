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
    final result = await _selectionWindow(
      selection: selection,
      query: query,
      minimumRevision: minimumRevision,
      maxItems: 64,
      fallbackOrdinal: fallbackOrdinal,
    );
    if (result == null) return null;
    final page = result.snapshot;
    final resolvedLocationId =
        page.queryAnchorResolution?.locationId ?? selection.locationId;
    final index = page.assets.indexWhere(
      (asset) => asset.locationId == resolvedLocationId,
    );
    final target = index + step;
    if (index < 0 || target < 0 || target >= page.assets.length) return null;
    final start = page.queryAnchorResolution?.windowStartItemOffset ?? 0;
    return (asset: page.assets[target], ordinal: start + target);
  }

  Future<int?> position({
    required LibraryAsset selection,
    required LibraryGalleryQuery query,
    required String queryId,
    required BigInt? revision,
  }) async {
    final result = await _selectionWindow(
      selection: selection,
      query: query,
      minimumRevision: revision,
      maxItems: 1,
      fallbackOrdinal: 0,
    );
    if (result == null ||
        result.snapshot.revision != revision ||
        result.snapshot.queryId != queryId) {
      return null;
    }
    return result.snapshot.queryAnchorResolution?.ordinal;
  }

  Future<LibraryQuerySnapshot?> _selectionWindow({
    required LibraryAsset selection,
    required LibraryGalleryQuery query,
    required BigInt? minimumRevision,
    required int maxItems,
    required int fallbackOrdinal,
  }) async {
    final direct = await _readWindow(
      query,
      minimumRevision,
      maxItems,
      LibraryQueryAnchor(requestedLocationId: selection.locationId),
    );
    if (direct.snapshot.queryAnchorResolution?.didResolve == true) {
      return direct;
    }

    final relocated = await _readWindow(
      query,
      minimumRevision,
      maxItems,
      LibraryQueryAnchor(
        requestedLocationId: selection.locationId,
        assetId: selection.assetId,
        fallbackGlobalItemIndex: fallbackOrdinal,
      ),
    );
    final resolved = relocated.snapshot.queryAnchorResolution?.locationId;
    final stillSelected = relocated.snapshot.assets.any(
      (asset) =>
          asset.locationId == resolved && asset.assetId == selection.assetId,
    );
    return stillSelected ? relocated : null;
  }

  Future<LibraryQuerySnapshot> _readWindow(
    LibraryGalleryQuery query,
    BigInt? minimumRevision,
    int maxItems,
    LibraryQueryAnchor anchor,
  ) async {
    final result = await _catalog.loadQuerySnapshot(
      maxItems: maxItems,
      query: query,
      anchor: anchor,
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
    return result;
  }
}
