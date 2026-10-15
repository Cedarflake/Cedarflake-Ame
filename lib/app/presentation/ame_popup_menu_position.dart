import "package:flutter/material.dart";

({RelativeRect position, double maximumHeight})? amePopupMenuAboveAnchor({
  required BuildContext context,
  required BuildContext anchorContext,
  required double menuHeight,
  double verticalGap = 4,
  double? viewportRightMargin,
}) {
  final overlayObject = Overlay.of(
    context,
    rootOverlay: true,
  ).context.findRenderObject();
  final anchorObject = anchorContext.findRenderObject();
  if (overlayObject is! RenderBox || anchorObject is! RenderBox) return null;
  final anchorTopRight = anchorObject.localToGlobal(
    Offset(anchorObject.size.width, 0),
    ancestor: overlayObject,
  );
  // Match the native popup route's screen inset; constrain the menu viewport
  // when its declared entries cannot all fit above the triggering control.
  final minimumTop = MediaQuery.paddingOf(context).top + 8;
  final availableHeight = anchorTopRight.dy - verticalGap - minimumTop;
  if (availableHeight <= 0) return null;
  final height = menuHeight.clamp(0.0, availableHeight).toDouble();
  final top = anchorTopRight.dy - verticalGap - height;
  return (
    position: RelativeRect.fromLTRB(
      overlayObject.size.width,
      top,
      viewportRightMargin ?? overlayObject.size.width - anchorTopRight.dx,
      overlayObject.size.height - top,
    ),
    maximumHeight: height,
  );
}

RelativeRect? amePopupMenuBelowAnchor({
  required BuildContext context,
  required BuildContext anchorContext,
  double verticalGap = 4,
  double? viewportRightMargin,
}) {
  final overlayObject = Overlay.of(
    context,
    rootOverlay: true,
  ).context.findRenderObject();
  final anchorObject = anchorContext.findRenderObject();
  if (overlayObject is! RenderBox || anchorObject is! RenderBox) {
    return null;
  }
  final anchorBottomRight = anchorObject.localToGlobal(
    Offset(anchorObject.size.width, anchorObject.size.height),
    ancestor: overlayObject,
  );
  final right =
      viewportRightMargin ?? overlayObject.size.width - anchorBottomRight.dx;
  final top = anchorBottomRight.dy + verticalGap;
  return RelativeRect.fromLTRB(
    overlayObject.size.width,
    top,
    right,
    overlayObject.size.height > top ? overlayObject.size.height - top : 0,
  );
}

RelativeRect? amePopupMenuAtGlobalPosition({
  required BuildContext context,
  required Offset globalPosition,
}) {
  final overlayObject = Overlay.of(
    context,
    rootOverlay: true,
  ).context.findRenderObject();
  if (overlayObject is! RenderBox) {
    return null;
  }
  final position = overlayObject.globalToLocal(globalPosition);
  return RelativeRect.fromLTRB(
    position.dx,
    position.dy,
    overlayObject.size.width - position.dx,
    overlayObject.size.height - position.dy,
  );
}
