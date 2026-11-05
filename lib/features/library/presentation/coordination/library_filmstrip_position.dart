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
    double extent,
  ) {
    if (state.assets.isEmpty) return;
    final index = (offset / extent).floor().clamp(0, state.assets.length - 1);
    final asset = state.assets[index];
    _query = state.query;
    _selection = selection.locationId;
    _fraction = offset / extent - index;
    _anchor = LibraryQueryAnchor(
      requestedLocationId: asset.locationId,
      assetId: asset.assetId,
      fallbackGlobalItemIndex: state.windowStartItemOffset + index,
    );
  }

  LibraryQueryAnchor? read(LibraryState state, LibraryAsset selection) =>
      state.query == _query && selection.locationId == _selection
      ? _anchor
      : null;

  double? offsetIn(LibraryState state, LibraryAsset selection, double extent) {
    final anchor = read(state, selection);
    if (anchor == null) return null;
    final index = state.assets.indexWhere(
      (asset) => asset.locationId == anchor.requestedLocationId,
    );
    return index < 0 ? null : (index + _fraction) * extent;
  }

  void clear() {
    _anchor = null;
    _query = null;
    _selection = null;
  }
}
