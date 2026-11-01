import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_review.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "library_classification_review.dart";
import "library_controller.dart";

final libraryClassificationAnnotationsProvider =
    FutureProvider.autoDispose<Map<String, LibraryClassificationReviewReason>>((
      ref,
    ) async {
      final window = ref.watch(
        libraryControllerProvider.select(
          (state) => _ClassificationWindow(
            state.query,
            state.catalogRevision,
            state.query.collection is LibrarySmartAlbumCollection
                ? state.assets
                      .map(LibraryOrganizationTarget.fromAsset)
                      .toList(growable: false)
                : const [],
          ),
        ),
      );
      final revision = window.revision;
      if (revision == null ||
          window.query.collection is! LibrarySmartAlbumCollection) {
        return const {};
      }
      final service = ref.watch(libraryClassificationReviewServiceProvider);
      var retired = false;
      ref.onDispose(() => retired = true);
      final result = <String, LibraryClassificationReviewReason>{};
      for (
        var offset = 0;
        offset < window.targets.length && !retired;
        offset += 256
      ) {
        final end = (offset + 256).clamp(0, window.targets.length);
        result.addAll(
          await service.annotations(
            revision,
            window.targets.sublist(offset, end),
          ),
        );
      }
      return retired ? const {} : Map.unmodifiable(result);
    });

class _ClassificationWindow {
  const _ClassificationWindow(this.query, this.revision, this.targets);
  final LibraryGalleryQuery query;
  final BigInt? revision;
  final List<LibraryOrganizationTarget> targets;

  @override
  bool operator ==(Object other) =>
      other is _ClassificationWindow &&
      query == other.query &&
      revision == other.revision &&
      listEquals(targets, other.targets);

  @override
  int get hashCode => Object.hash(query, revision, Object.hashAll(targets));
}
