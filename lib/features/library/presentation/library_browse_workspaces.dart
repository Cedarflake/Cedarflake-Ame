import "../application/library_organization_publication.dart";
import "../application/library_workbench_controller.dart";
import "../domain/library_models.dart";
import "library_organization_mode.dart";

typedef _WorkspaceBrowse = ({
  LibraryReviewBrowseState browse,
  LibraryReviewBrowseState? ordinary,
});

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
  final Map<LibraryWorkspace, _WorkspaceBrowse> _saved = {};
  LibraryWorkspace _browsing = LibraryWorkspace.gallery;
  LibraryWorkspace _visible = LibraryWorkspace.gallery;
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
    if (!destination.isBrowsing) {
      _visible = destination;
      publish(destination);
      return;
    }
    final retainsActiveGallery = destination == _browsing && review.isActive;
    if (destination == _visible || retainsActiveGallery) {
      _visible = destination;
      publish(destination);
      return;
    }
    final leaving = _captureBrowse();
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
        surface.library.query.collection is! LibraryAllCollection) {
      _saved[_browsing] = leaving;
    }
    final saved = _saved[destination];
    if (saved != null) {
      final restored = await surface.showReviewQuery(
        saved.browse.query,
        position: saved.browse.position,
      );
      if (_disposed) return;
      if (!restored) {
        _failed = (destination: destination, query: saved.browse.query);
        return;
      }
      if (!identical(_saved[destination], saved)) {
        final collection = saved.browse.query.collection;
        if (collection is LibraryAlbumCollection) {
          await review.retireAlbum(collection.albumId);
        }
        return;
      }
      if (saved.ordinary case final ordinary?) {
        if (!review.restoreWorkspace(ordinary)) {
          _failed = (destination: destination, query: saved.browse.query);
          return;
        }
      }
    }
    if (_disposed) return;
    surface.restoreReviewBrowse(saved?.browse ?? _captureBrowse().browse);
    _browsing = destination;
    _visible = destination;
    publish(destination);
  }

  _WorkspaceBrowse _captureBrowse() {
    final current = surface.captureBrowse();
    if (review.isActive) {
      return (browse: current, ordinary: review.ordinaryBrowse);
    }
    return (
      browse: LibraryReviewBrowseState(
        query: current.query,
        selection: current.selection.clear(),
        isSelecting: false,
      ),
      ordinary: null,
    );
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
          saved.browse.query.collection ==
          LibraryGalleryCollection.album(albumId),
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
