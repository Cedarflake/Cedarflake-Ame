import "package:flutter/widgets.dart";

import "library_gallery_reflow.dart";

class LibraryGalleryVisiblePosition {
  const LibraryGalleryVisiblePosition({
    required this.queryId,
    required this.revision,
    required this.monthKey,
    required this.locationId,
    this.assetId,
    required this.globalItemIndex,
    required this.itemFraction,
    required this.viewportFraction,
  });

  final String queryId;
  final BigInt revision;
  final String? monthKey;
  final String locationId;
  final String? assetId;
  final int globalItemIndex;
  final double itemFraction;
  final double viewportFraction;
}

class LibraryGalleryPositionResolver {
  String? _queryId;
  BigInt? _revision;
  LibraryGalleryVisiblePosition? Function(
    double scrollOffset,
    double viewportDimension,
  )?
  _resolve;

  void update({
    required String queryId,
    required BigInt? revision,
    required LibraryGalleryVisiblePosition? Function(
      double scrollOffset,
      double viewportDimension,
    )
    resolve,
  }) {
    _queryId = queryId;
    _revision = revision;
    _resolve = resolve;
  }

  LibraryGalleryVisiblePosition? resolve({
    required String queryId,
    required BigInt? revision,
    required double scrollOffset,
    required double viewportDimension,
  }) {
    if (_queryId != queryId || _revision != revision) {
      return null;
    }
    return _resolve?.call(scrollOffset, viewportDimension);
  }
}

class LibraryGalleryLayoutTransition {
  LibraryGalleryLayoutTransition({
    required this.generation,
    required this.position,
    required ScrollPosition? scrollPosition,
  }) : _scrollOrigin = LibraryGalleryScrollOrigin(scrollPosition);

  final int generation;
  final LibraryGalleryVisiblePosition position;
  final LibraryGalleryScrollOrigin _scrollOrigin;

  bool retainsScrollPosition(ScrollPosition? current) =>
      _scrollOrigin.matches(current);

  LibraryGalleryVisiblePosition? resolvePosition({
    required ScrollPosition? current,
    required LibraryGalleryPositionResolver? resolver,
    required LibraryGalleryVisiblePosition? fallback,
  }) {
    if (retainsScrollPosition(current)) {
      return position;
    }
    final resolved =
        current != null && current.hasPixels && current.hasViewportDimension
        ? resolver?.resolve(
            queryId: position.queryId,
            revision: position.revision,
            scrollOffset: current.pixels,
            viewportDimension: current.viewportDimension,
          )
        : null;
    final candidate = resolved ?? fallback;
    if (candidate?.queryId != position.queryId ||
        candidate?.revision != position.revision) {
      return null;
    }
    return candidate;
  }

  LibraryGalleryViewportAnchor? anchorForScrollPosition(
    ScrollPosition? current,
  ) {
    if (!retainsScrollPosition(current)) {
      return null;
    }
    return LibraryGalleryViewportAnchor(
      queryId: position.queryId,
      revision: position.revision,
      locationId: position.locationId,
      globalItemIndex: position.globalItemIndex,
      itemFraction: position.itemFraction,
      viewportFraction: position.viewportFraction,
    );
  }
}
