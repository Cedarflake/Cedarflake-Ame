import "package:flutter/material.dart";

import "timeline_linear_projection.dart";
import "timeline_rail_model.dart";
import "timeline_rail_motion.dart";

class TimelineRailRanges extends StatelessWidget {
  const TimelineRailRanges({
    required this.anchors,
    required this.projection,
    required this.geometry,
    required this.endpointInset,
    required this.isInteracting,
    super.key,
  });

  final List<TimelineRailAnchor> anchors;
  final TimelineLinearProjection projection;
  final TimelineRailGeometry geometry;
  final double endpointInset;
  final bool isInteracting;

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: LayoutBuilder(
      builder: (context, constraints) {
        final extent = (constraints.maxHeight - endpointInset * 2).clamp(
          0.0,
          double.infinity,
        );
        return Stack(
          children: [
            for (final anchor in anchors)
              if (anchor.bucket.highlighted)
                TimelineRailPosition(
                  key: ValueKey("rail-range-${anchor.bucket.id}"),
                  geometry: geometry,
                  isInteracting: isInteracting,
                  rect: Rect.fromLTWH(
                    constraints.maxWidth - 5,
                    endpointInset + anchor.value * extent,
                    3,
                    (projection.offsetToValue(
                              (anchor.bucket.scrollOffset ?? 0) +
                                  anchor.bucket.contentExtent,
                            ) -
                            anchor.value) *
                        extent,
                  ),
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.primary,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
          ],
        );
      },
    ),
  );
}
