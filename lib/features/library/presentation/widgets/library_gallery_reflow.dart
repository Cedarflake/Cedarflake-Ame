import "package:flutter/widgets.dart";

import "library_gallery_layout.dart";
import "library_gallery_layout_snapshot.dart";

class LibraryGalleryViewportAnchor {
  const LibraryGalleryViewportAnchor({
    required this.queryId,
    required this.revision,
    required this.locationId,
    required this.globalItemIndex,
    required this.itemFraction,
    required this.viewportFraction,
    this.monthKey,
  });

  static const _topPadding = 18.0;

  final String queryId;
  final BigInt revision;
  final String locationId;
  final int globalItemIndex;
  final double itemFraction;
  final double viewportFraction;
  final String? monthKey;

  static LibraryGalleryViewportAnchor? capture(
    LibraryGalleryLayoutSnapshot snapshot, {
    required double scrollOffset,
    required double viewportExtent,
    required double viewportFraction,
  }) {
    if (viewportExtent <= 0 || snapshot.entries.isEmpty) {
      return null;
    }
    final anchorOffset = scrollOffset + viewportExtent * viewportFraction;
    final entryIndex = _nearestPhotoEntry(snapshot, anchorOffset);
    if (entryIndex == null) {
      return null;
    }
    final entry = snapshot.entries[entryIndex];
    final itemIndex =
        entry.startItemIndex +
        centerCellIndex(entry.cellWidths, snapshot.availableWidth);
    final rowOffset = snapshot.metrics.offsetForGlobalItemIndex(itemIndex);
    if (rowOffset == null) {
      return null;
    }
    return LibraryGalleryViewportAnchor(
      queryId: snapshot.manifest.queryId,
      revision: snapshot.manifest.revision,
      locationId: snapshot.manifest.locationIdAt(itemIndex),
      globalItemIndex: itemIndex,
      itemFraction: ((anchorOffset - rowOffset) / entry.rowHeight).clamp(
        0.0,
        1.0,
      ),
      viewportFraction: viewportFraction,
      monthKey: entry.monthKey,
    );
  }

  double? scrollOffsetFor(
    LibraryGalleryLayoutSnapshot snapshot,
    double viewportExtent,
  ) {
    if (snapshot.manifest.queryId != queryId ||
        snapshot.manifest.revision != revision ||
        snapshot.manifest.itemCount == 0) {
      return null;
    }
    final itemIndex = globalItemIndex.clamp(0, snapshot.manifest.itemCount - 1);
    if (snapshot.manifest.locationIdAt(itemIndex) != locationId) {
      return null;
    }
    final rowOffset = snapshot.metrics.offsetForGlobalItemIndex(itemIndex);
    if (rowOffset == null) {
      return null;
    }
    final entryIndex = snapshot.entryIndexForScrollOffset(
      (rowOffset - _topPadding).clamp(0, double.infinity).toDouble(),
    );
    if (entryIndex < 0) {
      return null;
    }
    final target =
        rowOffset +
        snapshot.entries[entryIndex].rowHeight * itemFraction -
        viewportExtent * viewportFraction;
    final maximum = (snapshot.metrics.contentExtent - viewportExtent).clamp(
      0,
      double.infinity,
    );
    return target.clamp(0, maximum).toDouble();
  }

  static int centerCellIndex(List<double> cellWidths, double availableWidth) {
    // The wall has 24 leading and 16 trailing pixels outside the row.
    final target = availableWidth * 0.5 - 4;
    var bestIndex = 0;
    var bestDistance = double.infinity;
    var leading = 0.0;
    for (var index = 0; index < cellWidths.length; index += 1) {
      final center = leading + cellWidths[index] * 0.5;
      final distance = (center - target).abs();
      if (distance < bestDistance) {
        bestIndex = index;
        bestDistance = distance;
      }
      leading += cellWidths[index] + LibraryGalleryLayoutEntry.spacing;
    }
    return bestIndex;
  }

  static int? _nearestPhotoEntry(
    LibraryGalleryLayoutSnapshot snapshot,
    double anchorOffset,
  ) {
    final initialIndex = snapshot.entryIndexForScrollOffset(
      (anchorOffset - _topPadding).clamp(0, double.infinity).toDouble(),
    );
    if (initialIndex < 0) {
      return null;
    }
    int? previousIndex;
    for (var index = initialIndex; index >= 0; index -= 1) {
      final entry = snapshot.entries[index];
      if (entry.itemCount > 0 && entry.rowHeight > 0) {
        previousIndex = index;
        break;
      }
    }
    int? nextIndex;
    for (
      var index = initialIndex;
      index < snapshot.entries.length;
      index += 1
    ) {
      final entry = snapshot.entries[index];
      if (entry.itemCount > 0 && entry.rowHeight > 0) {
        nextIndex = index;
        break;
      }
    }
    if (previousIndex == null) {
      return nextIndex;
    }
    if (nextIndex == null) {
      return previousIndex;
    }
    double distanceToEntry(int index) {
      final top = _topPadding + snapshot.entryStartOffsets[index];
      final bottom = top + snapshot.entries[index].rowHeight;
      if (anchorOffset < top) {
        return top - anchorOffset;
      }
      if (anchorOffset > bottom) {
        return anchorOffset - bottom;
      }
      return 0;
    }

    if (distanceToEntry(previousIndex) <= distanceToEntry(nextIndex)) {
      return previousIndex;
    }
    return nextIndex;
  }
}

class LibraryGalleryScrollOrigin {
  LibraryGalleryScrollOrigin(ScrollPosition? position)
    : _position = position,
      _pixels = position != null && position.hasPixels ? position.pixels : null;

  final ScrollPosition? _position;
  final double? _pixels;

  bool matches(ScrollPosition? position) {
    if (!identical(position, _position)) {
      return false;
    }
    final pixels = position != null && position.hasPixels
        ? position.pixels
        : null;
    return pixels == _pixels;
  }
}

/// A queued geometry change retains its anchor only until the viewport moves.
class LibraryGalleryPendingReflow {
  LibraryGalleryPendingReflow({
    required this._availableWidth,
    required this._viewportExtent,
    required this._anchor,
    required ScrollPosition? position,
    required this._transitionGeneration,
  }) : _scrollOrigin = LibraryGalleryScrollOrigin(position);

  double _availableWidth;
  double _viewportExtent;
  final LibraryGalleryViewportAnchor? _anchor;
  final LibraryGalleryScrollOrigin _scrollOrigin;
  final int? _transitionGeneration;

  double get availableWidth => _availableWidth;
  double get viewportExtent => _viewportExtent;

  void updateViewport(double availableWidth, double viewportExtent) {
    _availableWidth = availableWidth;
    _viewportExtent = viewportExtent;
  }

  LibraryGalleryViewportAnchor? resolveAnchor({
    required LibraryGalleryLayoutSnapshot snapshot,
    required ScrollPosition? position,
    required int? transitionGeneration,
    required LibraryGalleryViewportAnchor? transitionAnchor,
  }) {
    if (transitionGeneration != null &&
        transitionGeneration != _transitionGeneration &&
        transitionAnchor != null) {
      return transitionAnchor;
    }
    if (position == null || !position.hasViewportDimension) {
      return null;
    }
    if (_scrollOrigin.matches(position) &&
        transitionGeneration == _transitionGeneration) {
      return _anchor;
    }
    return LibraryGalleryViewportAnchor.capture(
      snapshot,
      scrollOffset: position.pixels,
      viewportExtent: position.viewportDimension,
      viewportFraction: _anchor?.viewportFraction ?? 0.5,
    );
  }
}
