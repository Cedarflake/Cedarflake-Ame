import "package:flutter/foundation.dart";

import "../application/library_catalog.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_viewer_navigation_window.dart";
import "library_viewer_source_state.dart";

enum LibraryViewerDirection { previous, next }

typedef LibraryViewerNavigation = ({
  int? position,
  bool canPrevious,
  bool canNext,
});

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
  LibraryViewerSourceState? _source;
  _NavigationRequest? _navigation;
  Object? _lookup;
  bool _isDisposed = false;
  int? _ordinal;
  late final _navigationWindow = LibraryViewerNavigationWindow(_catalog);

  LibraryViewerSelection? get selection => _selection;
  String? get assetId => _selection?.assetId;
  String? get locationId => _selection?.locationId;
  LibraryAsset? get retainedAsset => _source?.asset;
  LibraryViewerSourceNotice? get sourceNotice => _source?.notice;
  int? get ordinal => _ordinal;
  bool get isNavigating => _navigation != null && _owns(_navigation!);

  LibraryViewerNavigation navigation(int totalItems) {
    final selection = _selection;
    if (selection == null ||
        sourceNotice == LibraryViewerSourceNotice.missing) {
      return (position: null, canPrevious: false, canNext: false);
    }
    final state = _readLibrary();
    final index = _index(state, selection);
    final ordinal = index < 0 ? _ordinal : state.windowStartItemOffset + index;
    if (isNavigating) {
      return (
        position: ordinal == null ? null : ordinal + 1,
        canPrevious: false,
        canNext: false,
      );
    }
    if (index < 0) {
      return (
        position: ordinal == null ? null : ordinal + 1,
        canPrevious: ordinal != null && ordinal > 0,
        canNext: ordinal != null && ordinal + 1 < totalItems,
      );
    }
    return (
      position: ordinal! + 1,
      canPrevious: index > 0 || state.hasPreviousAssets,
      canNext: index < state.assets.length - 1 || state.hasMoreAssets,
    );
  }

  _QueryIdentity get _query {
    final state = _readLibrary();
    return (
      query: state.query,
      id: state.queryId,
      revision: state.catalogRevision,
    );
  }

  void open(LibraryAsset asset, {int? ordinal}) {
    if (_isDisposed) {
      return;
    }
    _selection = LibraryViewerSelection._(asset);
    final state = _readLibrary();
    final index = state.assets.indexWhere(
      (item) => item.locationId == asset.locationId,
    );
    _ordinal = index < 0 ? ordinal : state.windowStartItemOffset + index;
    _source = LibraryViewerSourceState(asset);
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
    _ordinal = null;
    _source = null;
    _navigation = null;
    _lookup = null;
    _onPreviewDemand(null);
    notifyListeners();
  }

  void _retainCatalogAsset(
    LibraryViewerSelection selection,
    LibraryAsset asset,
  ) {
    if (_isDisposed ||
        !identical(_selection, selection) ||
        (asset.assetId != selection.assetId &&
            asset.locationId != selection.locationId)) {
      return;
    }
    final source = _source;
    if (source == null) return;
    source.reconcile(asset);
    if (source.notice == null && asset.locationId != selection.locationId) {
      _selection = LibraryViewerSelection._(asset);
      _navigation = null;
    }
    _onPreviewDemand(source.notice == null ? source.asset : null);
    notifyListeners();
  }

  void reloadSource() {
    final replacement = _source?.replacement;
    if (replacement != null) open(replacement, ordinal: _ordinal);
  }

  Future<bool> navigate(LibraryViewerDirection direction) async {
    final selection = _selection;
    if (_isDisposed ||
        selection == null ||
        sourceNotice == LibraryViewerSourceNotice.missing ||
        isNavigating) {
      return false;
    }
    final request = _NavigationRequest(selection, _query);
    _navigation = request;
    notifyListeners();
    try {
      final step = direction == LibraryViewerDirection.previous ? -1 : 1;
      var state = _readLibrary();
      var index = _index(state, selection);
      if (index < 0) {
        final retained = retainedAsset;
        if (retained == null) return false;
        final neighbor = await _navigationWindow.neighbor(
          selection: retained,
          query: request.query.query,
          minimumRevision: request.query.revision,
          fallbackOrdinal: _ordinal ?? 0,
          step: step,
        );
        if (_owns(request) && neighbor != null) {
          open(neighbor.asset, ordinal: neighbor.ordinal);
          return true;
        }
        return false;
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
        return false;
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
        return false;
      }
      open(state.assets[target]);
      return true;
    } on Object catch (error) {
      if (_owns(request)) {
        _onNavigationError(error);
      }
      return false;
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
    asset ??= _readLibrary().assets
        .where(
          (item) =>
              item.locationId == selection.locationId &&
              item.assetId != selection.assetId,
        )
        .firstOrNull;
    if (asset == null) {
      _source?.reconcile(null);
      _navigation = null;
      _onPreviewDemand(null);
      notifyListeners();
    } else {
      _retainCatalogAsset(selection, asset);
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
    _source = null;
    _navigation = null;
    _lookup = null;
    if (hadSelection) {
      _onPreviewDemand(null);
    }
    super.dispose();
  }
}
