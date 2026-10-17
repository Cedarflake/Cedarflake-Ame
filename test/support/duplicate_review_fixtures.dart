import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";

LibraryAsset reviewAsset(String id) => LibraryAsset(
  assetId: "asset-$id",
  locationId: id,
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "$id.png",
  displayPath: id == "one" ? r"C:\fixture\one.png" : r"D:\fixture\two.png",
  relativePath: "$id.png",
  previewPath: "",
  previewStatus: LibraryPreviewStatus.failed,
  fileSize: BigInt.from(1024),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 240,
  height: 160,
);

const reviewGroup = LibraryDuplicateGroup(
  digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  byteLength: 1024,
  locationCount: 2,
  physicalFileCount: 2,
  representativeLocationId: "one",
  representativePath: r"C:\fixture\one.png",
);

class ReviewFixtureService implements LibraryDuplicateReview {
  ReviewFixtureService({this.showResults});
  final bool Function()? showResults;
  LibraryDuplicateDecision decision = const LibraryDuplicateDecision();
  bool failSave = false;
  LibraryDuplicateAvailability secondAvailability =
      LibraryDuplicateAvailability.offline;
  Completer<LibraryDuplicateReviewPage>? pendingGroups;
  Completer<LibraryDuplicateReviewLocations>? pendingMembers;
  Completer<LibraryDuplicateDecision>? pendingSave;
  int loads = 0;
  String? nextLocationId;
  final List<String?> memberCursors = [];

  LibraryDuplicateReviewGroup get entry => LibraryDuplicateReviewGroup(
    group: reviewGroup,
    representative: reviewAsset("one"),
    decision: decision,
  );
  LibraryDuplicateReviewLocations get members =>
      LibraryDuplicateReviewLocations(
        locations: [
          LibraryDuplicateReviewLocation(
            asset: reviewAsset("one"),
            availability: LibraryDuplicateAvailability.local,
          ),
          LibraryDuplicateReviewLocation(
            asset: reviewAsset("two"),
            availability: secondAvailability,
          ),
        ],
        decision: decision,
        nextLocationId: nextLocationId,
      );

  @override
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  }) async {
    loads++;
    if (pendingGroups case final pending?) return pending.future;
    return LibraryDuplicateReviewPage(
      totalGroups: (showResults?.call() ?? true) && ignored == decision.ignored
          ? 1
          : 0,
      groups: (showResults?.call() ?? true) && ignored == decision.ignored
          ? [entry]
          : [],
    );
  }

  @override
  Future<LibraryDuplicateReviewLocations> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    memberCursors.add(after);
    if (pendingMembers case final pending?) return pending.future;
    return LibraryDuplicateReviewLocations(
      locations: members.locations,
      decision: decision,
      nextLocationId: after == null ? nextLocationId : null,
    );
  }

  @override
  Future<LibraryDuplicateDecision> prefer(
    LibraryDuplicateGroup group,
    LibraryAsset? asset,
  ) async {
    if (failSave) throw StateError("controlled decision write failure");
    if (pendingSave case final pending?) return pending.future;
    return decision = LibraryDuplicateDecision(
      preferredLocationId: asset?.locationId,
      preferredAssetId: asset?.assetId,
      ignored: decision.ignored,
    );
  }

  @override
  Future<LibraryDuplicateDecision> ignore(
    LibraryDuplicateGroup group,
    bool ignored,
  ) async {
    if (failSave) throw StateError("controlled decision write failure");
    return decision = LibraryDuplicateDecision(
      preferredLocationId: decision.preferredLocationId,
      preferredAssetId: decision.preferredAssetId,
      ignored: ignored,
    );
  }
}

class ReviewFixtureLibrary extends LibraryController {
  ReviewFixtureLibrary({this.duplicateMode = LibraryGalleryDuplicateMode.all});
  final LibraryGalleryDuplicateMode duplicateMode;
  int refreshes = 0;
  final demands = <Object, List<String>>{};
  @override
  LibraryState build() =>
      LibraryState(query: LibraryGalleryQuery(duplicateMode: duplicateMode));
  @override
  Stream<void> watchPreview(String locationId) => const Stream.empty();
  @override
  LibraryAsset resolvePreview(LibraryAsset asset) => asset;
  @override
  void updateFilmstripPreviewDemand(
    Object owner,
    Iterable<LibraryAsset> visible,
    int previewEdge,
  ) {
    demands[owner] = visible.map((value) => value.locationId).toList();
  }

  @override
  void releaseFilmstripPreviewDemand(Object owner) {
    demands.remove(owner);
  }

  @override
  void updateViewerPreviewDemand(LibraryAsset? viewer) {}
  @override
  Future<bool> refreshCurrentQuery() async {
    refreshes++;
    return true;
  }
}
