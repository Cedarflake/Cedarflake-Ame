import "package:flutter/widgets.dart";

import "widgets/library_gallery_position.dart";
import "widgets/library_gallery_reflow.dart";

/// Keeps one image through consecutive layout choices until browsing moves on.
class LibraryGalleryLayoutContinuity {
  LibraryGalleryVisiblePosition? _anchor;
  LibraryGalleryScrollEdge? _scrollEdge;
  ScrollPosition? _scrollPosition;
  LibraryGalleryScrollOrigin? _publishedOrigin;
  int? _pendingGeneration;

  LibraryGalleryLayoutTransition begin({
    required int generation,
    required LibraryGalleryVisiblePosition current,
    required ScrollPosition? scrollPosition,
  }) {
    final anchor = _anchor;
    final canRetain =
        anchor != null &&
        anchor.queryId == current.queryId &&
        anchor.revision == current.revision &&
        scrollPosition != null &&
        identical(scrollPosition, _scrollPosition) &&
        (_pendingGeneration != null ||
            (_publishedOrigin?.matches(scrollPosition) ?? false));
    final next = canRetain ? anchor : current;
    final edge = canRetain
        ? _scrollEdge
        : LibraryGalleryScrollEdge.capture(scrollPosition);
    clear();
    _anchor = next;
    _scrollEdge = edge;
    _pendingGeneration = generation;
    _scrollPosition = scrollPosition;
    // Native layout corrections do not notify scroll listeners. User and
    // programmatic movement do, including input before layout acknowledgement.
    scrollPosition?.addListener(clear);
    return LibraryGalleryLayoutTransition(
      generation: generation,
      position: next,
      scrollPosition: scrollPosition,
      scrollEdge: edge,
    );
  }

  void complete(int generation) {
    if (_pendingGeneration != generation) {
      return;
    }
    _pendingGeneration = null;
    _publishedOrigin = LibraryGalleryScrollOrigin(_scrollPosition);
  }

  void clear() {
    _scrollPosition?.removeListener(clear);
    _scrollPosition = null;
    _anchor = null;
    _scrollEdge = null;
    _publishedOrigin = null;
    _pendingGeneration = null;
  }
}
