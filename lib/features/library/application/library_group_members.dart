import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_exact_duplicates.dart";
import "../domain/library_group_members.dart";
import "../domain/library_gallery_group.dart";
import "../domain/library_models.dart";
import "../domain/library_similar_review.dart";
import "library_catalog.dart";
import "library_duplicate_review.dart";
import "library_similar_review_controller.dart";
import "ports/library_catalog.dart";
import "ports/library_duplicate_review.dart";
import "ports/library_similar_review.dart";

final libraryGroupMembersProvider = Provider<LibraryGroupMembers>(
  (ref) => LibraryGroupMembers(
    ref.read(libraryGroupMemberCatalogProvider),
    ref.read(libraryDuplicateReviewProvider),
    ref.read(librarySimilarReviewProvider),
  ),
);

class LibraryGroupMembers {
  const LibraryGroupMembers(this.catalog, this.exact, this.similar);
  final LibraryGroupMemberCatalog catalog;
  final LibraryDuplicateReview exact;
  final LibrarySimilarReview similar;

  Future<LibraryGroupMemberPage> load({
    required LibraryGalleryGroupKey key,
    required LibraryGalleryQuery? query,
    String? after,
  }) => catalog.loadGroupMembers(
    query: query ?? const LibraryGalleryQuery(),
    group: key,
    after: after,
  );

  Future<void> prefer({
    required LibraryGalleryGroupKey key,
    required LibraryGroupMember member,
    required BigInt revision,
  }) async {
    switch (key) {
      case LibraryExactGalleryGroupKey(:final digest, :final byteLength):
        await exact.prefer(
          LibraryDuplicateGroup(
            digest: digest,
            byteLength: byteLength.toInt(),
            locationCount: 0,
            physicalFileCount: 0,
            representativeLocationId: member.asset.locationId,
            representativePath: member.asset.displayPath,
          ),
          member.preferred ? null : member.asset,
        );
      case LibrarySimilarGalleryGroupKey(:final groupId):
        await similar.prefer(
          LibrarySimilarContext(
            groupId: groupId,
            degree: LibrarySimilarityDegree.standard,
            revision: revision,
          ),
          LibrarySimilarMember(
            content: member.content!,
            asset: member.asset,
            availability: member.availability,
            locationCount: 1,
            distance: 0,
            preferred: member.preferred,
          ),
          !member.preferred,
        );
    }
  }
}
