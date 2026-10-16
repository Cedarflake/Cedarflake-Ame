import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/exact_duplicates.dart" as rust_api;
import "../domain/library_exact_duplicates.dart";
import "library_controller.dart";

abstract interface class LibraryGalleryDuplicates {
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  );
}

class RustLibraryGalleryDuplicates implements LibraryGalleryDuplicates {
  const RustLibraryGalleryDuplicates();

  @override
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  ) async {
    final page = await rust_api.loadGalleryDuplicates(
      revision: revision,
      locationIds: locationIds,
    );
    if (page.revision != revision) {
      throw StateError(
        "Duplicate annotations belong to a different gallery revision",
      );
    }
    return {
      for (final annotation in page.annotations)
        annotation.locationId: LibraryDuplicateGroup(
          digest: annotation.group.digest,
          byteLength: annotation.group.byteLength.toInt(),
          locationCount: annotation.group.locationCount.toInt(),
          physicalFileCount: annotation.group.physicalFileCount.toInt(),
          representativeLocationId: annotation.group.representativeLocationId,
          representativePath: annotation.group.representativePath,
        ),
    };
  }
}

final libraryGalleryDuplicatesProvider = Provider<LibraryGalleryDuplicates?>(
  (ref) => null,
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
