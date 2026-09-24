import "library_models.dart";

class LibraryTimeIntent {
  const LibraryTimeIntent({required this.monthKey, required this.itemOffset});

  final String? monthKey;
  final int itemOffset;
}

class LibraryTimeSnapshot {
  const LibraryTimeSnapshot({
    required this.snapshot,
    required this.timeline,
    required this.anchor,
    required this.windowStartItemOffset,
  });

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;
  final LibraryTimeAnchor? anchor;
  final int windowStartItemOffset;

  int? get targetItemOffset {
    final target = anchor;
    if (target == null) {
      return null;
    }
    var preceding = 0;
    for (final bucket in timeline.buckets) {
      if (bucket.monthKey == target.monthKey) {
        if (target.itemOffset < 0 || target.itemOffset >= bucket.itemCount) {
          return null;
        }
        return preceding + target.itemOffset;
      }
      preceding += bucket.itemCount;
    }
    return null;
  }
}
