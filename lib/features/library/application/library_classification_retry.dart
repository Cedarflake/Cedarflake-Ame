import "package:flutter_riverpod/flutter_riverpod.dart";

import "../adapters/rust_library_classification_retry.dart";
import "../domain/library_organization.dart";
import "../domain/library_classification_review.dart";
import "library_classification_configuration.dart";

enum LibraryClassificationRetryOutcome { unchanged, queued, queuedWhilePaused }

abstract interface class LibraryClassificationRetry {
  Future<LibraryClassificationRetryOutcome> retry(
    LibraryOrganizationTarget target,
    BigInt expectedDispatchRevision,
  );
}

final libraryClassificationRetryProvider = Provider<LibraryClassificationRetry>(
  (ref) => const RustLibraryClassificationRetry(),
);

bool canRetryLibraryClassification(
  LibraryClassificationItem? item,
  LibraryClassificationSettings? settings,
) =>
    item?.reviewReason == LibraryClassificationReviewReason.analysisFailed &&
    (settings?.mode == LibraryClassificationMode.automatic ||
        settings?.mode == LibraryClassificationMode.paused);
