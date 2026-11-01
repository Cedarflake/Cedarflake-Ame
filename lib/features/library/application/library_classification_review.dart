import "package:flutter_riverpod/flutter_riverpod.dart";

import "../adapters/rust_library_classification_review.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";
import "library_controller.dart";

abstract interface class LibraryClassificationReviewService {
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  );
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  );
}

final libraryClassificationReviewServiceProvider =
    Provider<LibraryClassificationReviewService>(
      (ref) => const RustLibraryClassificationReview(),
    );

final libraryClassificationItemProvider = FutureProvider.autoDispose
    .family<LibraryClassificationItem?, LibraryOrganizationTarget>((
      ref,
      target,
    ) async {
      final revision = ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      if (revision == null) return null;
      final items = await ref
          .watch(libraryClassificationReviewServiceProvider)
          .loadItems([target]);
      return items.firstOrNull;
    });
