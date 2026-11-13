import "package:flutter/foundation.dart";

import "../../application/library_browse_admission.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";

enum LibraryFilmstripPage { selection, previous, next }

/// Owns one strip request and requires visible progress before automatic paging continues.
class LibraryFilmstripPaging extends ChangeNotifier {
  LibraryFilmstripPaging({
    required this.controller,
    required this.readState,
    required this.readSelection,
    required this.loadPrevious,
  });

  final LibraryController controller;
  final LibraryState Function() readState;
  final LibraryAsset Function() readSelection;
  final Future<void> Function() loadPrevious;
  Object? _request;
  bool _isDisposed = false;
  bool hasFailure = false;
  LibraryFilmstripPage? page;

  bool get isLoading => _request != null;

  void clearFailure() => hasFailure = false;

  void selectionChanged() {
    if (page != LibraryFilmstripPage.selection) return;
    _request = null;
    hasFailure = false;
  }

  Future<bool> load(LibraryFilmstripPage page, {bool retry = false}) async {
    final before = readState();
    if (_isDisposed ||
        isLoading ||
        (hasFailure && !retry) ||
        !LibraryBrowseAdmission(before).canReadPage) {
      return false;
    }
    final request = Object();
    final selected = readSelection();
    _request = request;
    hasFailure = false;
    this.page = page;
    notifyListeners();
    bool isCurrent() =>
        !_isDisposed &&
        identical(_request, request) &&
        readState().query == before.query &&
        (page != LibraryFilmstripPage.selection ||
            readSelection().galleryEntryId == selected.galleryEntryId);
    try {
      switch (page) {
        case LibraryFilmstripPage.selection:
          await controller.updateQuery(
            before.query,
            anchorLocationId: selected.locationId,
            anchorAssetId: selected.assetId,
            anchorGroupKey: selected.galleryGroup?.key,
            forceRefresh: true,
            showRefreshingStatus: false,
          );
        case LibraryFilmstripPage.previous:
          await loadPrevious();
        case LibraryFilmstripPage.next:
          await controller.loadNextPage();
      }
      if (!isCurrent()) return false;
      final current = readState();
      hasFailure = switch (page) {
        LibraryFilmstripPage.selection => !current.assets.any(
          (asset) => asset.galleryEntryId == selected.galleryEntryId,
        ),
        LibraryFilmstripPage.previous =>
          current.previousPageErrorMessage != null ||
              (current.hasPreviousAssets &&
                  current.windowStartItemOffset >=
                      before.windowStartItemOffset),
        LibraryFilmstripPage.next =>
          current.pageErrorMessage != null ||
              (current.hasMoreAssets &&
                  current.windowStartItemOffset + current.assets.length <=
                      before.windowStartItemOffset + before.assets.length),
      };
      return !hasFailure && page == LibraryFilmstripPage.selection;
    } on Object {
      if (isCurrent()) hasFailure = true;
      return false;
    } finally {
      if (!_isDisposed && identical(_request, request)) {
        _request = null;
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _request = null;
    super.dispose();
  }
}
