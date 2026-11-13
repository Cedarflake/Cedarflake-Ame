import "package:flutter/widgets.dart";

import "../../domain/gallery_layout_manifest.dart";
import "library_gallery_reflow.dart";

class LibraryGalleryVisiblePosition {
  const LibraryGalleryVisiblePosition({
    required this.queryId,
    required this.revision,
    required this.monthKey,
    required this.locationId,
    String? entryId,
    this.assetId,
    required this.globalItemIndex,
    required this.itemFraction,
    required this.viewportFraction,
    this.scrollEdge,
  }) : entryId = entryId ?? locationId;

  final String queryId;
  final BigInt revision;
  final String? monthKey;
  final String locationId;
  final String entryId;
  final String? assetId;
  final int globalItemIndex;
  final double itemFraction;
  final double viewportFraction;
  final LibraryGalleryScrollEdge? scrollEdge;

  LibraryGalleryVisiblePosition withScrollEdge(
    LibraryGalleryScrollEdge? edge,
  ) => LibraryGalleryVisiblePosition(
    queryId: queryId,
    revision: revision,
    monthKey: monthKey,
    locationId: locationId,
    entryId: entryId,
    assetId: assetId,
    globalItemIndex: globalItemIndex,
    itemFraction: itemFraction,
    viewportFraction: viewportFraction,
    scrollEdge: edge,
  );
}

LibraryGalleryVisiblePosition? rebaseLibraryGalleryPosition(
  LibraryGalleryVisiblePosition position,
  LibraryGalleryLayoutManifest manifest,
) {
  if (position.queryId != manifest.queryId || manifest.itemCount == 0) {
    return null;
  }
  var index = position.globalItemIndex.clamp(0, manifest.itemCount - 1);
  if (manifest.entryIdAt(index) != position.entryId) {
    for (var candidate = 0; candidate < manifest.itemCount; candidate++) {
      if (manifest.entryIdAt(candidate) == position.entryId) {
        index = candidate;
        break;
      }
    }
  }
  final locationId = manifest.locationIdAt(index);
  final date = manifest.dateKeyAt(index);
  return LibraryGalleryVisiblePosition(
    queryId: manifest.queryId,
    revision: manifest.revision,
    monthKey: date != null && date.length >= 7 ? date.substring(0, 7) : null,
    locationId: locationId,
    entryId: manifest.entryIdAt(index),
    assetId: locationId == position.locationId ? position.assetId : null,
    globalItemIndex: index,
    itemFraction: position.itemFraction,
    viewportFraction: position.viewportFraction,
    scrollEdge: position.scrollEdge,
  );
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
    LibraryGalleryLayoutManifest? manifest,
  }) {
    if (_queryId != queryId) {
      return null;
    }
    if (_revision == revision) {
      return _resolve?.call(scrollOffset, viewportDimension);
    }
    if (manifest == null ||
        manifest.queryId != queryId ||
        manifest.revision != revision ||
        _revision == null ||
        _revision! > manifest.revision) {
      return null;
    }
    final displayed = _resolve?.call(scrollOffset, viewportDimension);
    return displayed == null
        ? null
        : rebaseLibraryGalleryPosition(displayed, manifest);
  }
}

class LibraryGalleryLayoutTransition {
  LibraryGalleryLayoutTransition({
    required this.generation,
    required this.position,
    required ScrollPosition? scrollPosition,
    this.scrollEdge,
  }) : _scrollOrigin = LibraryGalleryScrollOrigin(scrollPosition);

  final int generation;
  final LibraryGalleryVisiblePosition position;
  final LibraryGalleryScrollEdge? scrollEdge;
  final LibraryGalleryScrollOrigin _scrollOrigin;

  bool retainsScrollPosition(ScrollPosition? current) =>
      _scrollOrigin.matches(current);

  LibraryGalleryScrollEdge? edgeForScrollPosition(ScrollPosition? current) =>
      retainsScrollPosition(current) ? scrollEdge : null;

  double constrainScrollOffset({
    required ScrollPosition? current,
    required double imageOffset,
    required double maximum,
  }) =>
      edgeForScrollPosition(current)?.offsetFor(maximum) ??
      imageOffset.clamp(0, maximum).toDouble();

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
      entryId: position.entryId,
      globalItemIndex: position.globalItemIndex,
      itemFraction: position.itemFraction,
      viewportFraction: position.viewportFraction,
      scrollEdge: scrollEdge,
    );
  }
}
