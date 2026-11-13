import "../../domain/library_models.dart";
import "../../domain/library_query_snapshot.dart";
import "../../domain/library_state.dart";

/// Retains the browsed strip position independently of the image being viewed.
class LibraryFilmstripPosition {
  LibraryGalleryQuery? _query;
  String? _selection;
  LibraryQueryAnchor? _anchor;
  double _fraction = 0;

  void capture(
    LibraryState state,
    LibraryAsset selection,
    double offset,
    double extent, {
    int leadingItems = 0,
  }) {
    if (state.assets.isEmpty) return;
    final assetOffset = offset / extent - leadingItems;
    final index = assetOffset.floor().clamp(0, state.assets.length - 1);
    final asset = state.assets[index];
    _query = state.query;
    _selection = selection.galleryEntryId;
    _fraction = assetOffset - index;
    _anchor = LibraryQueryAnchor(
      requestedLocationId: asset.locationId,
      groupKey: asset.galleryGroup?.key,
      assetId: asset.assetId,
      fallbackGlobalItemIndex: state.windowStartItemOffset + index,
    );
  }

  LibraryQueryAnchor? read(LibraryState state, LibraryAsset selection) =>
      state.query == _query && selection.galleryEntryId == _selection
      ? _anchor
      : null;

  double? offsetIn(
    LibraryState state,
    LibraryAsset selection,
    double extent, {
    int leadingItems = 0,
  }) {
    final anchor = read(state, selection);
    if (anchor == null) return null;
    final index = state.assets.indexWhere(
      (asset) =>
          asset.galleryEntryId ==
          (anchor.groupKey?.entryId ?? anchor.requestedLocationId),
    );
    return index < 0 ? null : (index + _fraction + leadingItems) * extent;
  }

  void clear() {
    _anchor = null;
    _query = null;
    _selection = null;
  }
}
