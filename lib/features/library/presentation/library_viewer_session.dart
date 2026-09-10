import "package:flutter/foundation.dart";

import "../application/library_catalog.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";

enum LibraryViewerDirection { previous, next }

class LibraryViewerSelection {
  LibraryViewerSelection._(LibraryAsset asset)
    : assetId = asset.assetId,
      locationId = asset.locationId;

  final String assetId;
  final String locationId;
}

typedef _QueryIdentity = ({
  LibraryGalleryQuery query,
  String id,
  BigInt? revision,
});

class _NavigationRequest {
  _NavigationRequest(this.selection, this.query);
  final LibraryViewerSelection selection;
  final _QueryIdentity query;
}

/// Owns user selection separately from the lifetime of catalog page requests.
class LibraryViewerSession extends ChangeNotifier {
  LibraryViewerSession({
    required this._readLibrary,
    required this._loadPrevious,
    required this._loadNext,
    required this._catalog,
    required this._onPreviewDemand,
    required this._onNavigationError,
  });

  final LibraryState Function() _readLibrary;
  final Future<void> Function() _loadPrevious;
  final Future<void> Function() _loadNext;
  final LibraryCatalog _catalog;
  final void Function(LibraryAsset?) _onPreviewDemand;
  final void Function(Object) _onNavigationError;
  LibraryViewerSelection? _selection;
  LibraryAsset? _retainedAsset;
  _NavigationRequest? _navigation;
  Object? _lookup;
  bool _isDisposed = false;

  LibraryViewerSelection? get selection => _selection;
  String? get assetId => _selection?.assetId;
  String? get locationId => _selection?.locationId;
  LibraryAsset? get retainedAsset => _retainedAsset;
  bool get isNavigating => _navigation != null && _owns(_navigation!);

  _QueryIdentity get _query {
    final state = _readLibrary();
    return (
      query: state.query,
      id: state.queryId,
      revision: state.catalogRevision,
    );
  }

  void open(LibraryAsset asset) {
    if (_isDisposed) {
      return;
    }
    _selection = LibraryViewerSelection._(asset);
    _retainedAsset = asset;
    _navigation = null;
    _lookup = null;
    _onPreviewDemand(asset);
    notifyListeners();
  }

  void close() {
    if (_isDisposed || _selection == null) {
      return;
    }
    _selection = null;
    _retainedAsset = null;
    _navigation = null;
    _lookup = null;
    _onPreviewDemand(null);
    notifyListeners();
  }

  void retainCatalogAsset(
    LibraryViewerSelection selection,
    LibraryAsset asset,
  ) {
    if (_isDisposed ||
        !identical(_selection, selection) ||
        asset.assetId != selection.assetId) {
      return;
    }
    if (asset.locationId != selection.locationId) {
      open(asset);
      return;
    }
    _retainedAsset = asset;
    _onPreviewDemand(asset);
  }

  Future<void> navigate(LibraryViewerDirection direction) async {
    final selection = _selection;
    if (_isDisposed || selection == null || isNavigating) {
      return;
    }
    final request = _NavigationRequest(selection, _query);
    _navigation = request;
    notifyListeners();
    try {
      final step = direction == LibraryViewerDirection.previous ? -1 : 1;
      var state = _readLibrary();
      var index = _index(state, selection);
      if (index < 0) {
        return;
      }
      var target = index + step;
      final requestedPage =
          (target < 0 && state.hasPreviousAssets) ||
          (target >= state.assets.length && state.hasMoreAssets);
      if (target < 0 && state.hasPreviousAssets) {
        await _loadPrevious();
      } else if (target >= state.assets.length && state.hasMoreAssets) {
        await _loadNext();
      }
      if (!_owns(request)) {
        return;
      }
      state = _readLibrary();
      index = _index(state, selection);
      target = index + step;
      if (index < 0 || target < 0 || target >= state.assets.length) {
        final message = direction == LibraryViewerDirection.previous
            ? state.previousPageErrorMessage
            : state.pageErrorMessage;
        if (message != null || requestedPage) {
          _onNavigationError(
            LibraryCatalogFailure(
              code: "viewer_page_load_failed",
              message: message ?? "图库窗口已变化，请重试切换图片。",
            ),
          );
        }
        return;
      }
      open(state.assets[target]);
    } on Object catch (error) {
      if (_owns(request)) {
        _onNavigationError(error);
      }
    } finally {
      if (!_isDisposed &&
          identical(_selection, selection) &&
          identical(_navigation, request)) {
        _navigation = null;
        notifyListeners();
      }
    }
  }

  Future<void> reconcile() async {
    final selection = _selection;
    final catalog = _catalog;
    if (_isDisposed ||
        selection == null ||
        catalog is! LibraryStableAssetCatalog) {
      return;
    }
    final stableCatalog = catalog as LibraryStableAssetCatalog;
    final lookup = Object();
    final query = _query;
    _lookup = lookup;
    LibraryAsset? asset;
    try {
      asset = await stableCatalog.loadAssetById(
        assetId: selection.assetId,
        preferredLocationId: selection.locationId,
      );
    } on Object {
      return;
    }
    if (_isDisposed ||
        !identical(_selection, selection) ||
        !identical(_lookup, lookup) ||
        query != _query) {
      return;
    }
    _lookup = null;
    if (asset == null) {
      close();
    } else {
      if (asset.locationId != selection.locationId) {
        open(asset);
      } else {
        retainCatalogAsset(selection, asset);
        notifyListeners();
      }
    }
  }

  bool _owns(_NavigationRequest request) =>
      !_isDisposed &&
      identical(_selection, request.selection) &&
      identical(_navigation, request) &&
      request.query == _query;

  int _index(LibraryState state, LibraryViewerSelection selection) =>
      state.assets.indexWhere(
        (asset) =>
            asset.assetId == selection.assetId &&
            asset.locationId == selection.locationId,
      );

  @override
  void dispose() {
    final hadSelection = _selection != null;
    _isDisposed = true;
    _selection = null;
    _retainedAsset = null;
    _navigation = null;
    _lookup = null;
    if (hadSelection) {
      _onPreviewDemand(null);
    }
    super.dispose();
  }
}
