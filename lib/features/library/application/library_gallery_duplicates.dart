import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_exact_duplicates.dart";
import "library_controller.dart";

import "ports/library_gallery_duplicates.dart";

final libraryGalleryDuplicatesProvider = Provider<LibraryGalleryDuplicates?>(
  (ref) => throw StateError(
    "LibraryGalleryDuplicates must be bound during application startup",
  ),
);

/// A bounded catalog window owns one batched request, shared by all its photo tiles.
final libraryGalleryDuplicateAnnotationsProvider =
    FutureProvider.autoDispose<Map<String, LibraryDuplicateGroup>>((ref) async {
      final service = ref.watch(libraryGalleryDuplicatesProvider);
      if (service == null) {
        return const {};
      }
      final window = ref.watch(
        libraryControllerProvider.select(
          (state) => _GalleryDuplicateWindow(
            state.catalogRevision,
            state.assets
                .map((asset) => asset.locationId)
                .toList(growable: false),
          ),
        ),
      );
      final revision = window.revision;
      if (revision == null || window.locationIds.isEmpty) {
        return const {};
      }
      var retired = false;
      ref.onDispose(() => retired = true);
      final groups = <String, LibraryDuplicateGroup>{};
      for (
        var offset = 0;
        offset < window.locationIds.length && !retired;
        offset += 256
      ) {
        final end = (offset + 256).clamp(0, window.locationIds.length);
        final page = await service.load(
          revision,
          window.locationIds.sublist(offset, end),
        );
        if (retired) {
          return const {};
        }
        groups.addAll(page);
      }
      return Map.unmodifiable(groups);
    });

class _GalleryDuplicateWindow {
  const _GalleryDuplicateWindow(this.revision, this.locationIds);
  final BigInt? revision;
  final List<String> locationIds;

  @override
  bool operator ==(Object other) =>
      other is _GalleryDuplicateWindow &&
      revision == other.revision &&
      listEquals(locationIds, other.locationIds);

  @override
  int get hashCode => Object.hash(revision, Object.hashAll(locationIds));
}
