import "dart:math" as math;
import "dart:ui" show lerpDouble;

import "package:flutter/rendering.dart";
import "package:flutter/widgets.dart";

/// Aligns measured header cells while the header moves between one and two action rows.
class LibraryGalleryHeaderRows extends MultiChildRenderObjectWidget {
  static const rowSpacing = 4.0;

  LibraryGalleryHeaderRows({
    required Widget title,
    required Widget summary,
    required Widget primaryActions,
    required Widget secondaryActions,
    required this.progress,
    super.key,
  }) : super(children: [title, summary, primaryActions, secondaryActions]);

  final double progress;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderLibraryGalleryHeaderRows(progress);

  @override
  void updateRenderObject(
    BuildContext context,
    RenderLibraryGalleryHeaderRows renderObject,
  ) {
    renderObject.progress = progress;
  }
}

class _HeaderCellParentData extends ContainerBoxParentData<RenderBox> {}

class RenderLibraryGalleryHeaderRows extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _HeaderCellParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _HeaderCellParentData> {
  RenderLibraryGalleryHeaderRows(this._progress);

  double _progress;

  set progress(double value) {
    if (_progress == value) return;
    _progress = value;
    markNeedsLayout();
  }

  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! _HeaderCellParentData) {
      child.parentData = _HeaderCellParentData();
    }
  }

  @override
  void performLayout() {
    final cells = getChildrenAsList();
    final [title, summary, primary, secondary] = cells;
    final width = constraints.maxWidth;
    final leadingConstraints = BoxConstraints(maxWidth: width * 0.55);
    title.layout(leadingConstraints, parentUsesSize: true);
    summary.layout(leadingConstraints, parentUsesSize: true);
    final leadingWidth = math.max(title.size.width, summary.size.width);
    final actionWidth = width - leadingWidth - 24;
    final actionConstraints = BoxConstraints.tightFor(width: actionWidth);
    primary.layout(actionConstraints, parentUsesSize: true);
    secondary.layout(actionConstraints, parentUsesSize: true);

    final titleHeight = title.size.height;
    final summaryHeight = summary.size.height;
    final labelHeight = titleHeight + 4 + summaryHeight;
    final collapsedHeight = math.max(
      70.0,
      math.max(labelHeight, primary.size.height),
    );
    final firstRowHeight = math.max(titleHeight, primary.size.height);
    final secondRowHeight = math.max(summaryHeight, secondary.size.height);
    final expandedHeight = math.max(
      70.0,
      firstRowHeight + LibraryGalleryHeaderRows.rowSpacing + secondRowHeight,
    );
    final height = lerpDouble(collapsedHeight, expandedHeight, _progress)!;
    size = constraints.constrain(Size(width, height));

    final collapsedTitleTop = (collapsedHeight - labelHeight) / 2;
    _position(title, 0, collapsedTitleTop, (firstRowHeight - titleHeight) / 2);
    _position(
      summary,
      0,
      collapsedTitleTop + titleHeight + 4,
      firstRowHeight +
          LibraryGalleryHeaderRows.rowSpacing +
          (secondRowHeight - summaryHeight) / 2,
    );
    _position(
      primary,
      width - actionWidth,
      (collapsedHeight - primary.size.height) / 2,
      (firstRowHeight - primary.size.height) / 2,
    );
    _position(
      secondary,
      width - actionWidth,
      collapsedHeight - secondary.size.height,
      firstRowHeight +
          LibraryGalleryHeaderRows.rowSpacing +
          (secondRowHeight - secondary.size.height) / 2,
    );
  }

  void _position(
    RenderBox child,
    double left,
    double collapsedTop,
    double expandedTop,
  ) {
    final parentData = child.parentData! as _HeaderCellParentData;
    parentData.offset = Offset(
      left,
      lerpDouble(collapsedTop, expandedTop, _progress)!,
    );
  }

  @override
  void paint(PaintingContext context, Offset offset) =>
      defaultPaint(context, offset);

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);
}
