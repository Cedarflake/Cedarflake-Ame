import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

import "timeline_rail_model.dart";

class TimelineRailGeometry {
  TimelineRailGeometry({
    required this.extent,
    required this.maximumOffset,
    required List<TimelineRailAnchor> anchors,
  }) : positions = [
         for (final anchor in anchors) (anchor.bucket.id, anchor.value),
       ];

  final double extent;
  final double maximumOffset;
  final List<(String, double)> positions;

  @override
  bool operator ==(Object other) =>
      other is TimelineRailGeometry &&
      extent == other.extent &&
      maximumOffset == other.maximumOffset &&
      listEquals(positions, other.positions);

  @override
  int get hashCode =>
      Object.hash(extent, maximumOffset, Object.hashAll(positions));
}

/// Layout changes move annotations; scroll input keeps the current line immediate.
class TimelineRailPosition extends StatefulWidget {
  const TimelineRailPosition({
    required this.rect,
    required this.geometry,
    required this.child,
    this.isInteracting = false,
    super.key,
  });

  final Rect rect;
  final TimelineRailGeometry geometry;
  final bool isInteracting;
  final Widget child;

  @override
  State<TimelineRailPosition> createState() => _TimelineRailPositionState();
}

class _TimelineRailPositionState extends State<TimelineRailPosition> {
  bool _animatePosition = true;

  @override
  void didUpdateWidget(covariant TimelineRailPosition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.rect != oldWidget.rect) {
      _animatePosition = widget.geometry != oldWidget.geometry;
    }
  }

  @override
  Widget build(BuildContext context) {
    final skipMotion =
        widget.isInteracting ||
        MediaQuery.disableAnimationsOf(context) ||
        !TickerMode.valuesOf(context).enabled;
    if (skipMotion) {
      return Positioned.fromRect(rect: widget.rect, child: widget.child);
    }
    return AnimatedPositioned.fromRect(
      rect: widget.rect,
      duration: _animatePosition
          ? const Duration(milliseconds: 180)
          : Duration.zero,
      curve: Curves.easeInOutCubic,
      child: widget.child,
    );
  }
}
