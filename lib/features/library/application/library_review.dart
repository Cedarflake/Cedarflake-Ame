import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_review.dart";

final libraryReviewProvider = Provider<LibraryReview>((ref) {
  throw StateError("LibraryReview must be bound during application startup");
});
