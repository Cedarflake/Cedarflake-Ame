import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_duplicate_review.dart";

final libraryDuplicateReviewProvider = Provider<LibraryDuplicateReview>((ref) {
  throw StateError(
    "LibraryDuplicateReview must be bound during application startup",
  );
});
