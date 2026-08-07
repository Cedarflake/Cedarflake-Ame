import "package:flutter/material.dart";

import "annotated_time_rail.dart";

class GalleryTimeNavigationState {
  const GalleryTimeNavigationState({this.timelineValue = 0});

  final double timelineValue;

  GalleryTimeNavigationState withTimelineValue(double value) {
    return GalleryTimeNavigationState(
      timelineValue: value.clamp(0.0, 1.0).toDouble(),
    );
  }

  GalleryTimeNavigationState publish({
    required List<TimelineRailBucket> buckets,
    required String bucketId,
  }) {
    if (buckets.isEmpty) {
      return this;
    }
    final nextTimelineValue = timelineRailValueForBucket(buckets, bucketId);
    if ((nextTimelineValue - timelineValue).abs() <= 0.0001) {
      return this;
    }
    return GalleryTimeNavigationState(timelineValue: nextTimelineValue);
  }
}

class LibraryTimeNavigation extends StatelessWidget {
  const LibraryTimeNavigation({
    required this.isLoading,
    required this.buckets,
    required this.navigationState,
    required this.onNavigationStateChanged,
    required this.onBucketActivated,
    super.key,
  });

  final bool isLoading;
  final List<TimelineRailBucket> buckets;
  final GalleryTimeNavigationState navigationState;
  final ValueChanged<GalleryTimeNavigationState> onNavigationStateChanged;
  final ValueChanged<TimelineRailBucket> onBucketActivated;

  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return const SizedBox(
        width: 128,
        child: Center(
          child: SizedBox.square(
            dimension: 24,
            child: CircularProgressIndicator(strokeWidth: 3),
          ),
        ),
      );
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (buckets.isNotEmpty)
          AnnotatedTimeRail(
            key: const Key("library-time-rail"),
            value: navigationState.timelineValue,
            buckets: buckets,
            onChanged: (value) => onNavigationStateChanged(
              navigationState.withTimelineValue(value),
            ),
            onBucketActivated: onBucketActivated,
          ),
      ],
    );
  }
}
