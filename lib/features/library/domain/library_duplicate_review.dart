import "library_exact_duplicates.dart";
import "library_models.dart";

class LibraryDuplicateDecision {
  const LibraryDuplicateDecision({
    this.preferredLocationId,
    this.preferredAssetId,
    this.ignored = false,
  });
  final String? preferredLocationId;
  final String? preferredAssetId;
  final bool ignored;

  bool prefers(LibraryAsset asset) =>
      preferredLocationId == asset.locationId &&
      preferredAssetId == asset.assetId;
}

class LibraryDuplicateReviewGroup {
  const LibraryDuplicateReviewGroup({
    required this.group,
    required this.representative,
    required this.decision,
  });
  final LibraryDuplicateGroup group;
  final LibraryAsset representative;
  final LibraryDuplicateDecision decision;
}

class LibraryDuplicateReviewPage {
  const LibraryDuplicateReviewPage({
    required this.groups,
    required this.totalGroups,
    this.next,
  });
  final List<LibraryDuplicateReviewGroup> groups;
  final int totalGroups;
  final LibraryDuplicateGroupCursor? next;
}

class LibraryDuplicateReviewLocation {
  const LibraryDuplicateReviewLocation({
    required this.asset,
    required this.availability,
  });
  final LibraryAsset asset;
  final LibraryDuplicateAvailability availability;

  LibraryDuplicateLocation get location => LibraryDuplicateLocation(
    locationId: asset.locationId,
    displayPath: asset.displayPath,
    availability: availability,
  );
}

class LibraryDuplicateReviewLocations {
  const LibraryDuplicateReviewLocations({
    required this.locations,
    required this.decision,
    this.nextLocationId,
  });
  final List<LibraryDuplicateReviewLocation> locations;
  final LibraryDuplicateDecision decision;
  final String? nextLocationId;
}
