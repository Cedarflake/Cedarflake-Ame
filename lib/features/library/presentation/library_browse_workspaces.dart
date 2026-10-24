import "../application/library_organization_publication.dart";
import "../application/library_workbench_controller.dart";
import "../domain/library_models.dart";
import "library_organization_mode.dart";

/// Shares one gallery while retaining each browsing destination's context.
class LibraryBrowseWorkspaces implements LibraryOrganizationProjection {
  LibraryBrowseWorkspaces({
    required this.surface,
    required this.review,
    required this.publish,
  });

  final LibraryReviewSurface surface;
  final LibraryOrganizationMode review;
  final void Function(LibraryWorkspace) publish;
  final Map<LibraryWorkspace, LibraryReviewBrowseState> _saved = {};
  LibraryWorkspace _browsing = LibraryWorkspace.gallery;
  LibraryWorkspace? _requested;
  ({LibraryWorkspace destination, LibraryGalleryQuery query})? _failed;
  Future<void>? _switching;
  bool _disposed = false;

  Future<void> select(LibraryWorkspace destination) {
    if (_disposed) return Future.value();
    _requested = destination;
    return _switching ??= _drain();
  }

  Future<void> _drain() async {
    try {
      while (!_disposed && _requested != null) {
        final next = _requested!;
        _requested = null;
        await _show(next);
      }
    } finally {
      _switching = null;
    }
  }

  Future<void> _show(LibraryWorkspace destination) async {
    _failed = null;
    if (!destination.isBrowsing || destination == _browsing) {
      publish(destination);
      return;
    }
    if (review.isActive) {
      await review.exit();
      if (_disposed) return;
      if (review.isActive) {
        final ordinary = review.ordinaryQuery;
        if (ordinary != null) {
          _failed = (destination: destination, query: ordinary);
        }
        return;
      }
    }
    if (_browsing == LibraryWorkspace.gallery ||
        surface.library.query.collection is LibraryAlbumCollection) {
      _saved[_browsing] = surface.captureBrowse();
    }
    final saved = _saved[destination];
    if (saved != null) {
      final restored = await surface.showReviewQuery(
        saved.query,
        position: saved.position,
      );
      if (_disposed) return;
      if (!restored) {
        _failed = (destination: destination, query: saved.query);
        return;
      }
      if (!identical(_saved[destination], saved)) {
        final collection = saved.query.collection;
        if (collection is LibraryAlbumCollection) {
          await review.retireAlbum(collection.albumId);
        }
        return;
      }
      surface.restoreReviewBrowse(saved);
    }
    if (_disposed) return;
    _browsing = destination;
    publish(destination);
  }

  Future<bool> retry(LibraryGalleryQuery query) async {
    final failed = _failed;
    if (_disposed || failed == null || failed.query != query) return false;
    await select(failed.destination);
    return true;
  }

  @override
  Future<bool> retireAlbum(String albumId) async {
    _saved.removeWhere(
      (_, saved) =>
          saved.query.collection == LibraryGalleryCollection.album(albumId),
    );
    return !_disposed && await review.retireAlbum(albumId);
  }

  void dispose() {
    _disposed = true;
    _requested = null;
    _failed = null;
    _saved.clear();
  }
}
