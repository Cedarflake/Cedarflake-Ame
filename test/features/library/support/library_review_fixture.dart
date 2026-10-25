import "package:cedarflake_ame/features/library/application/library_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_position.dart";

import "../presentation/support/library_screen_fixture.dart";

typedef ReviewWrite = ({
  String id,
  LibraryReviewPosition position,
  LibraryOrganizationPrepared prepared,
  LibraryReviewAction action,
});

class ReviewServiceFixture implements LibraryReview {
  BigInt arrivalBoundary = BigInt.zero;

  @override
  Future<BigInt> admissionBoundary() async => arrivalBoundary;

  final starts = <LibraryGalleryQuery>[];
  final ends = <String>[];
  final continuationQueries = <LibraryGalleryQuery?>[];
  final checkpoints =
      <({LibraryReviewPosition position, LibraryOrganizationTarget target})>[];
  final writes = <ReviewWrite>[];
  Future<LibraryReviewPosition> Function()? starting;
  Future<LibraryReviewPosition> Function(LibraryReviewPosition)? checkpointing;
  Future<LibraryReviewReceipt> Function(ReviewWrite)? writing;
  Future<LibraryReviewContinuation> Function()? reading;
  LibraryReviewContinuation saved = const LibraryReviewContinuation(
    LibraryContinuationStatus.noActivity,
  );

  @override
  Future<LibraryReviewPosition> begin(LibraryGalleryQuery query) async {
    starts.add(query);
    return starting?.call() ??
        LibraryReviewPosition("session-${starts.length}", BigInt.zero);
  }

  @override
  Future<void> end(String sessionId) async => ends.add(sessionId);

  @override
  Future<LibraryReviewPosition> checkpoint(
    LibraryReviewPosition position,
    LibraryOrganizationTarget target,
  ) async {
    checkpoints.add((position: position, target: target));
    return checkpointing?.call(position) ?? position;
  }

  @override
  Future<LibraryReviewContinuation> continuation({
    LibraryGalleryQuery? query,
  }) async {
    continuationQueries.add(query);
    return reading?.call() ?? saved;
  }

  @override
  Future<LibraryReviewReceipt> change(
    String operationId,
    LibraryReviewPosition position,
    LibraryOrganizationPrepared prepared,
    LibraryReviewAction action,
  ) async {
    final write = (
      id: operationId,
      position: position,
      prepared: prepared,
      action: action,
    );
    writes.add(write);
    return writing?.call(write) ?? receipt(write);
  }

  LibraryReviewReceipt receipt(ReviewWrite write) => LibraryReviewReceipt(
    LibraryOrganizationReceipt(
      operationId: write.id,
      revision: BigInt.two,
      changedGroups: 1,
    ),
    write.position.next(),
  );
}

class ReviewSurfaceFixture implements LibraryReviewSurface {
  ReviewSurfaceFixture() : library = populatedScreenState(totalItems: 12);
  @override
  LibraryState library;
  LibraryReviewBrowseState? restored;
  final queries = <LibraryGalleryQuery>[];
  final targets = <LibraryOrganizationTarget?>[];
  final positions = <LibraryGalleryVisiblePosition?>[];
  final opened = <LibraryAsset>[];
  int closes = 0;
  Future<bool> Function()? publishing;

  @override
  LibraryReviewBrowseState captureBrowse() => LibraryReviewBrowseState(
    query: library.query,
    selection: GallerySelection.empty(
      library.queryId,
    ).toggle(library.assets.first.assetId),
    isSelecting: true,
    position: LibraryGalleryVisiblePosition(
      queryId: library.queryId,
      revision: library.catalogRevision!,
      monthKey: null,
      locationId: library.assets.first.locationId,
      assetId: library.assets.first.assetId,
      globalItemIndex: 23,
      itemFraction: .3,
      viewportFraction: .4,
    ),
  );

  @override
  Future<bool> showReviewQuery(
    LibraryGalleryQuery query, {
    LibraryOrganizationTarget? target,
    LibraryGalleryVisiblePosition? position,
  }) async {
    queries.add(query);
    targets.add(target);
    positions.add(position);
    final applies = await (publishing?.call() ?? Future.value(true));
    if (applies) library = library.copyWith(query: query);
    return applies;
  }

  @override
  void restoreReviewBrowse(LibraryReviewBrowseState saved) => restored = saved;
  @override
  void openReviewImage(LibraryAsset asset) => opened.add(asset);
  @override
  void closeReviewImage() => closes++;
}
