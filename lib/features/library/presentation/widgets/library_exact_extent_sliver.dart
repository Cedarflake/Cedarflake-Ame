import "package:flutter/rendering.dart";
import "package:flutter/widgets.dart";

class LibraryExactExtentSliver extends SliverMultiBoxAdaptorWidget {
  LibraryExactExtentSliver.builder({
    required this.itemStartOffsets,
    required this.contentExtent,
    required NullableIndexedWidgetBuilder itemBuilder,
    this.layoutCorrection,
    ChildIndexGetter? findChildIndexCallback,
    bool addAutomaticKeepAlives = true,
    bool addRepaintBoundaries = true,
    bool addSemanticIndexes = true,
    super.key,
  }) : assert(contentExtent >= 0),
       assert(
         itemStartOffsets.isEmpty ||
             (itemStartOffsets.first == 0 &&
                 itemStartOffsets.last <= contentExtent),
       ),
       super(
         delegate: SliverChildBuilderDelegate(
           itemBuilder,
           findChildIndexCallback: findChildIndexCallback,
           childCount: itemStartOffsets.length,
           addAutomaticKeepAlives: addAutomaticKeepAlives,
           addRepaintBoundaries: addRepaintBoundaries,
           addSemanticIndexes: addSemanticIndexes,
         ),
       );

  final List<double> itemStartOffsets;
  final double contentExtent;
  final LibraryExactExtentLayoutCorrection? layoutCorrection;

  @override
  RenderLibraryExactExtentSliver createRenderObject(BuildContext context) {
    return RenderLibraryExactExtentSliver(
      childManager: context as SliverMultiBoxAdaptorElement,
      itemStartOffsets: itemStartOffsets,
      contentExtent: contentExtent,
      layoutCorrection: layoutCorrection,
    );
  }

  @override
  void updateRenderObject(
    BuildContext context,
    RenderLibraryExactExtentSliver renderObject,
  ) {
    renderObject
      ..itemStartOffsets = itemStartOffsets
      ..contentExtent = contentExtent
      ..layoutCorrection = layoutCorrection;
  }
}

class LibraryExactExtentLayoutCorrection {
  const LibraryExactExtentLayoutCorrection({
    required this.generation,
    required this.delta,
  });

  final int generation;
  final double delta;
}

class RenderLibraryExactExtentSliver extends RenderSliverFixedExtentBoxAdaptor {
  RenderLibraryExactExtentSliver({
    required super.childManager,
    required this._itemStartOffsets,
    required this._contentExtent,
    required LibraryExactExtentLayoutCorrection? layoutCorrection,
  }) : _layoutCorrection = layoutCorrection,
       _pendingLayoutCorrection = layoutCorrection;

  List<double> get itemStartOffsets => _itemStartOffsets;
  List<double> _itemStartOffsets;
  set itemStartOffsets(List<double> value) {
    if (identical(value, _itemStartOffsets)) {
      return;
    }
    _itemStartOffsets = value;
    markNeedsLayout();
  }

  double get contentExtent => _contentExtent;
  double _contentExtent;
  set contentExtent(double value) {
    if (value == _contentExtent) {
      return;
    }
    _contentExtent = value;
    markNeedsLayout();
  }

  LibraryExactExtentLayoutCorrection? get layoutCorrection => _layoutCorrection;
  LibraryExactExtentLayoutCorrection? _layoutCorrection;
  LibraryExactExtentLayoutCorrection? _pendingLayoutCorrection;
  int? _appliedLayoutCorrectionGeneration;
  var _appliedLayoutCorrectionCount = 0;
  int get appliedLayoutCorrectionCount => _appliedLayoutCorrectionCount;

  set layoutCorrection(LibraryExactExtentLayoutCorrection? value) {
    if (value?.generation == _layoutCorrection?.generation) {
      return;
    }
    _layoutCorrection = value;
    if (value == null ||
        value.generation == _appliedLayoutCorrectionGeneration) {
      _pendingLayoutCorrection = null;
      return;
    }
    _pendingLayoutCorrection = value;
    markNeedsLayout();
  }

  @override
  void performLayout() {
    final correction = _pendingLayoutCorrection;
    if (correction != null) {
      _pendingLayoutCorrection = null;
      _appliedLayoutCorrectionGeneration = correction.generation;
      if (correction.delta.abs() >= 0.001) {
        _appliedLayoutCorrectionCount += 1;
        geometry = SliverGeometry(scrollOffsetCorrection: correction.delta);
        return;
      }
    }
    super.performLayout();
  }

  @override
  double? get itemExtent => null;

  @override
  ItemExtentBuilder get itemExtentBuilder => _itemExtentAt;

  double? _itemExtentAt(int index, SliverLayoutDimensions dimensions) {
    if (index < 0 || index >= _itemStartOffsets.length) {
      return null;
    }
    final endOffset = index + 1 < _itemStartOffsets.length
        ? _itemStartOffsets[index + 1]
        : _contentExtent;
    return endOffset - _itemStartOffsets[index];
  }

  @override
  double indexToLayoutOffset(double itemExtent, int index) {
    if (index <= 0 || _itemStartOffsets.isEmpty) {
      return 0;
    }
    if (index >= _itemStartOffsets.length) {
      return _contentExtent;
    }
    return _itemStartOffsets[index];
  }

  @override
  int getMinChildIndexForScrollOffset(double scrollOffset, double itemExtent) {
    if (_itemStartOffsets.isEmpty || scrollOffset <= 0) {
      return 0;
    }
    var lower = 0;
    var upper = _itemStartOffsets.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (_itemStartOffsets[middle] <= scrollOffset) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    return (lower - 1).clamp(0, _itemStartOffsets.length - 1);
  }

  @override
  int getMaxChildIndexForScrollOffset(double scrollOffset, double itemExtent) {
    if (_itemStartOffsets.isEmpty || scrollOffset <= 0) {
      return 0;
    }
    var lower = 0;
    var upper = _itemStartOffsets.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (_itemStartOffsets[middle] < scrollOffset) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    return (lower - 1).clamp(0, _itemStartOffsets.length - 1);
  }

  @override
  double estimateMaxScrollOffset(
    SliverConstraints constraints, {
    int? firstIndex,
    int? lastIndex,
    double? leadingScrollOffset,
    double? trailingScrollOffset,
  }) {
    return _contentExtent;
  }

  @override
  double computeMaxScrollOffset(
    SliverConstraints constraints,
    double itemExtent,
  ) {
    return _contentExtent;
  }
}
