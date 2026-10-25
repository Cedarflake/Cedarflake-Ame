import "package:flutter/material.dart";

class TimelineRailMarker extends StatelessWidget {
  const TimelineRailMarker({super.key});

  static const double extent = 4;
  static const double minimumGap = 6;

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: SizedBox.square(
      dimension: extent,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.outline,
          shape: BoxShape.circle,
        ),
      ),
    ),
  );
}

class TimelineRailTicks extends StatelessWidget {
  const TimelineRailTicks({required this.endpointInset, super.key});

  final double endpointInset;

  @override
  Widget build(BuildContext context) => ExcludeSemantics(
    child: IgnorePointer(
      child: LayoutBuilder(
        builder: (context, constraints) {
          final usableHeight = constraints.maxHeight - endpointInset * 2;
          if (usableHeight < TimelineRailMarker.extent) {
            return const SizedBox.shrink();
          }
          const minimumSpacing =
              TimelineRailMarker.extent + TimelineRailMarker.minimumGap;
          if (usableHeight < minimumSpacing) {
            return const Center(child: TimelineRailMarker());
          }
          final intervals = const [
            20,
            10,
            5,
            2,
            1,
          ].firstWhere((count) => usableHeight / count >= minimumSpacing);
          return Padding(
            padding: EdgeInsets.symmetric(
              vertical: endpointInset - TimelineRailMarker.extent / 2,
            ),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                for (var index = 0; index <= intervals; index++)
                  TimelineRailMarker(
                    key: ValueKey("position-tick-${index * 20 ~/ intervals}"),
                  ),
              ],
            ),
          );
        },
      ),
    ),
  );
}
