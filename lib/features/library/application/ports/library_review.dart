import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";

abstract interface class LibraryReview {
  Future<BigInt> admissionBoundary();
  Future<LibraryReviewPosition> begin(LibraryGalleryQuery query);
  Future<void> end(String sessionId);
  Future<LibraryReviewPosition> checkpoint(
    LibraryReviewPosition position,
    LibraryOrganizationTarget target,
  );
  Future<LibraryReviewContinuation> continuation({LibraryGalleryQuery? query});
  Future<LibraryReviewReceipt> change(
    String operationId,
    LibraryReviewPosition position,
    LibraryOrganizationPrepared prepared,
    LibraryReviewAction action,
  );
}
