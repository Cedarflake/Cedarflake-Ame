import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/exact_duplicate_review.dart" as api;
import "../../../src/rust/domain/exact_duplicate_review.dart" as dto;
import "../../../src/rust/domain/exact_duplicates.dart" as exact;
import "../domain/library_duplicate_review.dart";
import "../domain/library_exact_duplicates.dart";
import "../domain/library_models.dart";
import "library_catalog.dart";

abstract interface class LibraryDuplicateReview {
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  });
  Future<LibraryDuplicateReviewRange> seekGroups(
    int index, {
    required bool ignored,
  });
  Future<LibraryDuplicateReviewLocations> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  );
  Future<LibraryDuplicateDecision> prefer(
    LibraryDuplicateGroup group,
    LibraryAsset? asset,
  );
  Future<LibraryDuplicateDecision> ignore(
    LibraryDuplicateGroup group,
    bool ignored,
  );
}

class RustLibraryDuplicateReview implements LibraryDuplicateReview {
  const RustLibraryDuplicateReview();

  dto.ExactDuplicateReviewKey _key(LibraryDuplicateGroup group) =>
      dto.ExactDuplicateReviewKey(
        digest: group.digest,
        byteLength: BigInt.from(group.byteLength),
      );

  LibraryDuplicateDecision _decision(dto.ExactDuplicateDecision value) =>
      LibraryDuplicateDecision(
        preferredLocationId: value.preferredLocationId,
        preferredAssetId: value.preferredAssetId,
        ignored: value.ignored,
      );

  @override
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  }) async {
    final page = await api.loadExactDuplicateReviewGroups(
      after: after == null
          ? null
          : exact.ExactDuplicateGroupCursor(
              digest: after.digest,
              byteLength: BigInt.from(after.byteLength),
            ),
      ignored: ignored,
    );
    return _groups(page);
  }

  @override
  Future<LibraryDuplicateReviewRange> seekGroups(
    int index, {
    required bool ignored,
  }) async {
    final range = await api.seekExactDuplicateReviewGroups(
      index: BigInt.from(index),
      ignored: ignored,
    );
    return LibraryDuplicateReviewRange(
      startIndex: range.startIndex.toInt(),
      page: _groups(range.page),
    );
  }

  LibraryDuplicateReviewPage _groups(dto.ExactDuplicateReviewPage page) {
    return LibraryDuplicateReviewPage(
      totalGroups: page.totalGroups.toInt(),
      groups: page.groups
          .map(
            (value) => LibraryDuplicateReviewGroup(
              group: LibraryDuplicateGroup(
                digest: value.group.digest,
                byteLength: value.group.byteLength.toInt(),
                locationCount: value.group.locationCount.toInt(),
                physicalFileCount: value.group.physicalFileCount.toInt(),
                representativeLocationId: value.group.representativeLocationId,
                representativePath: value.group.representativePath,
              ),
              representative: mapRustLibraryAsset(value.representative),
              decision: _decision(value.decision),
            ),
          )
          .toList(growable: false),
      next: page.next == null
          ? null
          : LibraryDuplicateGroupCursor(
              page.next!.digest,
              page.next!.byteLength.toInt(),
            ),
    );
  }

  @override
  Future<LibraryDuplicateReviewLocations> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    final page = await api.loadExactDuplicateReviewLocations(
      key: _key(group),
      after: after,
    );
    return LibraryDuplicateReviewLocations(
      locations: page.locations
          .map(
            (value) => LibraryDuplicateReviewLocation(
              asset: mapRustLibraryAsset(value.asset),
              availability: switch (value.availability) {
                exact.DuplicateLocationAvailability.local =>
                  LibraryDuplicateAvailability.local,
                exact.DuplicateLocationAvailability.cloudOnly =>
                  LibraryDuplicateAvailability.cloudOnly,
                exact.DuplicateLocationAvailability.offline =>
                  LibraryDuplicateAvailability.offline,
                exact.DuplicateLocationAvailability.unavailable =>
                  LibraryDuplicateAvailability.unavailable,
              },
            ),
          )
          .toList(growable: false),
      decision: _decision(page.decision),
      nextLocationId: page.nextLocationId,
    );
  }

  @override
  Future<LibraryDuplicateDecision> prefer(
    LibraryDuplicateGroup group,
    LibraryAsset? asset,
  ) async => _decision(
    await api.setExactDuplicatePreference(
      key: _key(group),
      preferred: asset == null
          ? null
          : dto.ExactDuplicatePreference(
              locationId: asset.locationId,
              assetId: asset.assetId,
              sourceGeneration: asset.sourceGeneration,
            ),
    ),
  );

  @override
  Future<LibraryDuplicateDecision> ignore(
    LibraryDuplicateGroup group,
    bool ignored,
  ) async => _decision(
    await api.setExactDuplicateIgnored(key: _key(group), ignored: ignored),
  );
}

final libraryDuplicateReviewProvider = Provider<LibraryDuplicateReview>(
  (ref) => const RustLibraryDuplicateReview(),
);
