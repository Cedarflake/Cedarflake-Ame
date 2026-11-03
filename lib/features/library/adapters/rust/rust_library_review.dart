import "../../../../src/rust/api/organization_review.dart" as api;
import "../../../../src/rust/domain.dart" as rust;
import "../../../../src/rust/domain/organization.dart" as organization;
import "../../../../src/rust/domain/organization_review.dart" as dto;
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "library_gallery_query_mapping.dart";
import "rust_library_organization.dart";

import "../../application/ports/library_review.dart";

class RustLibraryReview implements LibraryReview {
  const RustLibraryReview();

  @override
  Future<BigInt> admissionBoundary() =>
      _call(api.loadOrganizationAdmissionBoundary);

  @override
  Future<LibraryReviewPosition> begin(LibraryGalleryQuery query) => _call(
    () async => _position(
      await api.beginOrganizationReview(
        query: mapLibraryGalleryQueryToRust(query),
      ),
    ),
  );

  @override
  Future<void> end(String sessionId) =>
      _call(() => api.endOrganizationReview(sessionId: sessionId));

  @override
  Future<LibraryReviewPosition> checkpoint(
    LibraryReviewPosition position,
    LibraryOrganizationTarget target,
  ) => _call(
    () async => _position(
      await api.checkpointOrganizationReview(
        position: _nativePosition(position),
        selection: organization.OrganizationSelection(
          assetId: target.assetId,
          locationId: target.locationId,
          sourceGeneration: target.sourceGeneration,
        ),
      ),
    ),
  );

  @override
  Future<LibraryReviewContinuation> continuation({
    LibraryGalleryQuery? query,
  }) => _call(() async {
    final value = await api.loadOrganizationContinuation(
      query: query == null ? null : mapLibraryGalleryQueryToRust(query),
    );
    final target = value.target;
    return LibraryReviewContinuation(
      switch (value.status) {
        dto.OrganizationContinuationStatus.noActivity =>
          LibraryContinuationStatus.noActivity,
        dto.OrganizationContinuationStatus.ready =>
          LibraryContinuationStatus.ready,
        dto.OrganizationContinuationStatus.anchorUnavailable =>
          LibraryContinuationStatus.anchorUnavailable,
        dto.OrganizationContinuationStatus.scopeUnavailable =>
          LibraryContinuationStatus.scopeUnavailable,
        dto.OrganizationContinuationStatus.emptyScope =>
          LibraryContinuationStatus.emptyScope,
        dto.OrganizationContinuationStatus.endOfQueue =>
          LibraryContinuationStatus.endOfQueue,
      },
      query: value.query == null ? null : mapRustGalleryQuery(value.query!),
      target: target == null
          ? null
          : LibraryOrganizationTarget(
              assetId: target.assetId,
              locationId: target.locationId,
              sourceGeneration: target.sourceGeneration,
            ),
    );
  });

  @override
  Future<LibraryReviewReceipt> change(
    String operationId,
    LibraryReviewPosition position,
    LibraryOrganizationPrepared prepared,
    LibraryReviewAction action,
  ) => _call(() async {
    if (prepared is! RustOrganizationPrepared) {
      throw StateError("The selection belongs to another organization adapter");
    }
    final result = await api.changeCurrentOrganizationReview(
      operationId: operationId,
      session: _nativePosition(position),
      prepared: prepared.value,
      action: switch (action) {
        LibraryReviewAlbum(:final album) =>
          album == null
              ? const organization.OrganizationAction.removeAlbum()
              : organization.OrganizationAction.assignAlbum(albumId: album.id),
        LibraryReviewProgress(:final action) => mapLibraryProgressAction(
          action,
        ),
      },
    );
    return LibraryReviewReceipt(
      LibraryOrganizationReceipt(
        operationId: result.operation.operationId,
        revision: result.operation.organizationRevision,
        changedGroups: result.operation.changedGroups.toInt(),
      ),
      _position(result.session),
    );
  });

  Future<T> _call<T>(Future<T> Function() operation) async {
    try {
      return await operation();
    } on rust.ScanError catch (error) {
      throw LibraryOrganizationFailure(error.code, error.message);
    }
  }

  LibraryReviewPosition _position(dto.OrganizationReviewSession value) =>
      LibraryReviewPosition(value.sessionId, value.positionSequence);

  dto.OrganizationReviewSession _nativePosition(LibraryReviewPosition value) =>
      dto.OrganizationReviewSession(
        sessionId: value.sessionId,
        positionSequence: value.sequence,
      );
}

organization.OrganizationAction mapLibraryProgressAction(
  LibraryProgressAction action,
) => switch (action) {
  LibraryProgressAction.defer => const organization.OrganizationAction.defer(),
  LibraryProgressAction.ignore =>
    const organization.OrganizationAction.ignore(),
  LibraryProgressAction.restore =>
    const organization.OrganizationAction.restorePending(),
};
