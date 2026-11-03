import "../../domain/library_classification_review.dart";
import "../../domain/library_organization.dart";

abstract interface class LibraryClassificationReviewService {
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  );
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  );
}
