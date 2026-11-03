import "../../domain/library_duplicate_review.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_models.dart";

abstract interface class LibraryDuplicateReview {
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  });
  Future<LibraryDuplicateReviewRange> seekGroups(
    int index, {
    required bool ignored,
  });
  Future<LibraryDuplicateReviewLocations> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  );
  Future<LibraryDuplicateDecision> prefer(
    LibraryDuplicateGroup group,
    LibraryAsset? asset,
  );
  Future<LibraryDuplicateDecision> ignore(
    LibraryDuplicateGroup group,
    bool ignored,
  );
}
