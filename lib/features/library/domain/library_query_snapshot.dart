import "library_models.dart";

class LibraryQueryAnchor {
  const LibraryQueryAnchor({
    required this.requestedLocationId,
    this.assetId,
    this.fallbackGlobalItemIndex = 0,
  });

  final String requestedLocationId;
  final String? assetId;
  final int fallbackGlobalItemIndex;
}

class LibraryQuerySnapshot {
  const LibraryQuerySnapshot({required this.snapshot, required this.timeline});

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;
}
