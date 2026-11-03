import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_duplicate_review.dart";
import "../../features/library/application/library_duplicate_review.dart";

List<Override> libraryDuplicateBindings() => [
  libraryDuplicateReviewProvider.overrideWithValue(
    const RustLibraryDuplicateReview(),
  ),
];

final libraryDuplicateRequirements = <ProviderListenable<Object?>>[
  libraryDuplicateReviewProvider,
];
