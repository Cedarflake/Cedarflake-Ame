import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_page_operation.dart";

class LibraryRetainedWindowUpdate {
  const LibraryRetainedWindowUpdate({
    required this.assets,
    required this.startItemOffset,
    required this.previousCursor,
    required this.nextCursor,
    required this.addedItemCount,
  });

  final List<LibraryAsset> assets;
  final int startItemOffset;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
  final int addedItemCount;
}

class _RetainedCatalogPage {
  const _RetainedCatalogPage({
    required this.assets,
    required this.startItemOffset,
    required this.previousCursor,
    required this.nextCursor,
  });

  final List<LibraryAsset> assets;
  final int startItemOffset;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
}

/// Retains whole catalog pages; the viewport admits reads and publishes results.
class LibraryRetainedCatalogWindow {
  static const _highWatermark = 5000;
  static const _lowWatermark = 3500;

  final List<_RetainedCatalogPage> _pages = [];

  void reset({
    required List<LibraryAsset> assets,
    required int startItemOffset,
    required LibraryCatalogCursor? previousCursor,
    required LibraryCatalogCursor? nextCursor,
  }) {
    _pages.clear();
    if (assets.isEmpty) {
      return;
    }
    _pages.add(
      _RetainedCatalogPage(
        assets: List.unmodifiable(assets),
        startItemOffset: startItemOffset,
        previousCursor: previousCursor,
        nextCursor: nextCursor,
      ),
    );
  }

  LibraryRetainedWindowUpdate merge(
    LibrarySnapshot snapshot, {
    required LibraryState current,
    required LibraryPageDirection direction,
  }) {
    _synchronize(current);
    final existingIds = {for (final asset in current.assets) asset.locationId};
    _mergeAssetUpdates(snapshot.assets);
    final addedAssets = [
      for (final asset in snapshot.assets)
        if (!existingIds.contains(asset.locationId)) asset,
    ];
    if (addedAssets.isNotEmpty) {
      final int startItemOffset;
      if (direction == LibraryPageDirection.next) {
        startItemOffset = current.windowStartItemOffset + current.assets.length;
      } else {
        startItemOffset = (current.windowStartItemOffset - addedAssets.length)
            .clamp(0, current.timeline?.totalItems ?? 0)
            .toInt();
      }
      final page = _RetainedCatalogPage(
        assets: List.unmodifiable(addedAssets),
        startItemOffset: startItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      if (direction == LibraryPageDirection.next) {
        _pages.add(page);
      } else {
        _pages.insert(0, page);
      }
    } else {
      _advanceBoundaryCursor(snapshot, direction);
    }
    _trim(trimLeading: direction == LibraryPageDirection.next);
    return LibraryRetainedWindowUpdate(
      assets: List.unmodifiable([for (final page in _pages) ...page.assets]),
      startItemOffset: _pages.first.startItemOffset,
      previousCursor: _pages.first.previousCursor,
      nextCursor: direction == LibraryPageDirection.next
          ? snapshot.nextCursor
          : _pages.last.nextCursor,
      addedItemCount: addedAssets.length,
    );
  }

  void _synchronize(LibraryState current) {
    final retainedCount = _pages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (_pages.isNotEmpty &&
        retainedCount == current.assets.length &&
        _pages.first.startItemOffset == current.windowStartItemOffset) {
      return;
    }
    reset(
      assets: current.assets,
      startItemOffset: current.windowStartItemOffset,
      previousCursor: current.previousCursor,
      nextCursor: current.nextCursor,
    );
  }

  void _mergeAssetUpdates(List<LibraryAsset> updates) {
    final replacements = {for (final asset in updates) asset.locationId: asset};
    if (replacements.isEmpty) {
      return;
    }
    for (var index = 0; index < _pages.length; index++) {
      final page = _pages[index];
      var didChange = false;
      final assets = [
        for (final asset in page.assets)
          replacements[asset.locationId] ?? asset,
      ];
      for (var assetIndex = 0; assetIndex < page.assets.length; assetIndex++) {
        if (!identical(page.assets[assetIndex], assets[assetIndex])) {
          didChange = true;
          break;
        }
      }
      if (didChange) {
        _pages[index] = _RetainedCatalogPage(
          assets: List.unmodifiable(assets),
          startItemOffset: page.startItemOffset,
          previousCursor: page.previousCursor,
          nextCursor: page.nextCursor,
        );
      }
    }
  }

  void _advanceBoundaryCursor(
    LibrarySnapshot snapshot,
    LibraryPageDirection direction,
  ) {
    if (_pages.isEmpty) {
      return;
    }
    if (direction == LibraryPageDirection.next) {
      final lastIndex = _pages.length - 1;
      final last = _pages[lastIndex];
      _pages[lastIndex] = _RetainedCatalogPage(
        assets: last.assets,
        startItemOffset: last.startItemOffset,
        previousCursor: last.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
    } else {
      final first = _pages.first;
      _pages[0] = _RetainedCatalogPage(
        assets: first.assets,
        startItemOffset: first.startItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: first.nextCursor,
      );
    }
  }

  void _trim({required bool trimLeading}) {
    var retainedCount = _pages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (retainedCount <= _highWatermark) {
      return;
    }
    while (_pages.length > 1 && retainedCount > _lowWatermark) {
      final removed = trimLeading ? _pages.removeAt(0) : _pages.removeLast();
      retainedCount -= removed.assets.length;
    }
  }
}
