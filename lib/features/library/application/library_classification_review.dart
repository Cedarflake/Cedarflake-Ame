import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";
import "library_controller.dart";
import "ports/library_classification_review.dart";

final libraryClassificationReviewServiceProvider =
    Provider<LibraryClassificationReviewService>(
      (ref) =>
          throw StateError("Library classification review binding is required"),
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
