import "library_models.dart";
import "gallery_layout_manifest.dart";

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
  const LibraryQuerySnapshot({
    required this.snapshot,
    required this.timeline,
    this.organizationCounts,
    this.layoutManifest,
  });

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;
  final LibraryOrganizationCounts? organizationCounts;
  final LibraryGalleryLayoutManifest? layoutManifest;
  LibraryOrganizationCountSnapshot? get organizationSummary =>
      organizationCounts == null
      ? null
      : LibraryOrganizationCountSnapshot(
          snapshot.queryId,
          snapshot.revision,
          organizationCounts!,
        );
}
