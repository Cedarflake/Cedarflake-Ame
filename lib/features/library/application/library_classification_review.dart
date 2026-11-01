import "package:flutter_riverpod/flutter_riverpod.dart";

import "../adapters/rust_library_classification_review.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";

abstract interface class LibraryClassificationReviewService {
  Future<LibraryClassificationDraft> open(LibraryClassificationScope scope);
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  );
  Future<LibraryClassificationReceipt> undo(
    String operationId,
    String originalOperationId,
  );
}

abstract interface class LibraryClassificationDraft {
  Future<void> appendTargets(List<LibraryOrganizationTarget> targets);
  Future<void> append(List<String> ids);
  Future<LibraryClassificationSummary> prepare();
  Future<LibraryClassificationReceipt> apply(
    String operationId,
    LibraryClassificationCommand command,
  );
  Future<void> close();
}

final libraryClassificationReviewServiceProvider =
    Provider<LibraryClassificationReviewService>(
      (ref) => const RustLibraryClassificationReview(),
    );
