import "package:flutter/material.dart";

import "library_viewer_zoom.dart";

enum LibraryComparisonSide { left, right }

class LibraryComparisonView extends ChangeNotifier {
  LibraryComparisonView({required TickerProvider vsync})
    : left = LibraryViewerZoom(vsync: vsync),
      right = LibraryViewerZoom(vsync: vsync) {
    left.transform.addListener(_leftChanged);
    right.transform.addListener(_rightChanged);
  }

  final LibraryViewerZoom left;
  final LibraryViewerZoom right;
  LibraryComparisonSide _active = LibraryComparisonSide.left;
  bool _linked = true;
  bool _synchronizing = false;

  LibraryComparisonSide get active => _active;
  bool get linked => _linked;
  LibraryViewerZoom get activeZoom =>
      _active == LibraryComparisonSide.left ? left : right;

  void select(LibraryComparisonSide side) {
    if (_active == side) return;
    _active = side;
    notifyListeners();
  }

  void toggleLinked() {
    _linked = !_linked;
    if (_linked) {
      _synchronize(
        activeZoom,
        _active == LibraryComparisonSide.left ? right : left,
      );
    }
    notifyListeners();
  }

  void fitBoth() {
    _synchronizing = true;
    left.fit();
    right.fit();
    _synchronizing = false;
  }

  void _leftChanged() => _changed(left, right);
  void _rightChanged() => _changed(right, left);

  void _changed(LibraryViewerZoom source, LibraryViewerZoom target) {
    if (_synchronizing || source.isUpdatingLayout) return;
    _synchronize(source, target);
    notifyListeners();
  }

  void _synchronize(LibraryViewerZoom source, LibraryViewerZoom target) {
    if (!_linked) return;
    _synchronizing = true;
    try {
      target.follow(source);
    } finally {
      _synchronizing = false;
    }
  }

  @override
  void dispose() {
    left.transform.removeListener(_leftChanged);
    right.transform.removeListener(_rightChanged);
    left.dispose();
    right.dispose();
    super.dispose();
  }
}
