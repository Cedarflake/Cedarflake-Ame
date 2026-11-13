import "library_asset.dart";
import "library_exact_duplicates.dart";
import "library_similar_review.dart";

class LibraryGroupMember {
  const LibraryGroupMember({
    required this.asset,
    required this.preferred,
    required this.availability,
    this.content,
  });
  final LibraryAsset asset;
  final bool preferred;
  final LibraryDuplicateAvailability availability;
  final LibrarySimilarContent? content;

  LibraryDuplicateLocation get location => LibraryDuplicateLocation(
    locationId: asset.locationId,
    displayPath: asset.displayPath,
    availability: availability,
  );
}

class LibraryGroupMemberPage {
  const LibraryGroupMemberPage({
    required this.members,
    required this.total,
    required this.revision,
    this.pending = 0,
    this.next,
  });
  final List<LibraryGroupMember> members;
  final int total;
  final int pending;
  final BigInt revision;
  final String? next;
}
