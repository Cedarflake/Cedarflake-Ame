import "package:flutter/widgets.dart";

import "library_gallery_layout.dart";
import "library_virtual_gallery_geometry.dart";

/// Only window-local geometry needs compensation when earlier rows arrive.
/// A full manifest already owns every row's position, including unloaded rows.
class LibraryGalleryPrependCompensation {
  LibraryGalleryPrependCompensation._({
    required this._position,
    required this._queryId,
    required this._revision,
    required this._anchorItemIndex,
    required this._anchorOffset,
  });

  final ScrollPosition _position;
  final String? _queryId;
  final BigInt? _revision;
  final int? _anchorItemIndex;
  final double? _anchorOffset;
  bool _isRetired = false;

  bool get isActive => !_isRetired;

  void retire() => _isRetired = true;

  static LibraryGalleryPrependCompensation capture({
    required ScrollPosition position,
    required LibraryGalleryLayoutMetrics? metrics,
    required LibraryVirtualGalleryGeometry? geometry,
    required BigInt? revision,
  }) {
    int? anchorIndex;
    double? anchorOffset;
    if (metrics != null && geometry != null && !metrics.isQueryWide) {
      anchorIndex = geometry.windowStartItemOffset;
      final offset = metrics.offsetForGlobalItemIndex(anchorIndex);
      if (offset != null) {
        anchorOffset = offset + geometry.leadingExtent;
      }
    }
    return LibraryGalleryPrependCompensation._(
      position: position,
      queryId: geometry?.queryId,
      revision: revision,
      anchorItemIndex: anchorIndex,
      anchorOffset: anchorOffset,
    );
  }

  double? resolve({
    required ScrollPosition position,
    required LibraryGalleryLayoutMetrics metrics,
    required LibraryVirtualGalleryGeometry geometry,
    required BigInt? revision,
  }) {
    final anchorIndex = _anchorItemIndex;
    final anchorOffset = _anchorOffset;
    if (_isRetired ||
        anchorIndex == null ||
        anchorOffset == null ||
        !identical(position, _position) ||
        metrics.isQueryWide ||
        geometry.queryId != _queryId ||
        revision != _revision ||
        geometry.windowStartItemOffset >= anchorIndex) {
      return null;
    }
    final offset = metrics.offsetForGlobalItemIndex(anchorIndex);
    if (offset == null) {
      return null;
    }
    final displacement = offset + geometry.leadingExtent - anchorOffset;
    return (position.pixels + displacement)
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
  }
}
