import "../../../src/rust/api/classification_progress.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/classification_review.dart" as dto;
import "../../../src/rust/domain/organization.dart" as organization;
import "../application/library_catalog.dart";
import "../application/library_classification_progress.dart";
import "../application/library_gallery_query_mapping.dart";
import "../domain/library_classification_progress.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";

class RustLibraryClassificationProgress
    implements LibraryClassificationProgress {
  const RustLibraryClassificationProgress();

  @override
  Future<LibraryClassificationPosition> begin(LibraryGalleryQuery query) =>
      _call(() async {
        final value = await api.beginClassificationReview(
          query: mapLibraryGalleryQueryToRust(query),
        );
        return LibraryClassificationPosition(value.sessionId, value.sequence);
      });

  @override
  Future<void> checkpoint(
    LibraryClassificationPosition position,
    LibraryOrganizationTarget target,
  ) => _call(
    () => api.checkpointClassificationReview(
      position: api.ClassificationReviewPosition(
        sessionId: position.sessionId,
        sequence: position.sequence,
      ),
      target: organization.OrganizationSelection(
        assetId: target.assetId,
        locationId: target.locationId,
        sourceGeneration: target.sourceGeneration,
      ),
    ),
  );

  @override
  Future<void> end(String sessionId) =>
      _call(() => api.endClassificationReview(sessionId: sessionId));

  @override
  Future<LibraryClassificationContinuation> continuation() => _call(() async {
    final value = await api.loadClassificationContinuation();
    final target = value.target;
    return LibraryClassificationContinuation(
      switch (value.status) {
        dto.ClassificationContinuationStatus.noActivity =>
          LibraryClassificationContinuationStatus.noActivity,
        dto.ClassificationContinuationStatus.ready =>
          LibraryClassificationContinuationStatus.ready,
        dto.ClassificationContinuationStatus.anchorUnavailable =>
          LibraryClassificationContinuationStatus.anchorUnavailable,
        dto.ClassificationContinuationStatus.scopeUnavailable =>
          LibraryClassificationContinuationStatus.scopeUnavailable,
        dto.ClassificationContinuationStatus.emptyScope =>
          LibraryClassificationContinuationStatus.emptyScope,
        dto.ClassificationContinuationStatus.endOfScope =>
          LibraryClassificationContinuationStatus.endOfScope,
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
}

Future<T> _call<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on rust.ScanError catch (error) {
    throw LibraryClassificationFailure(error.code, error.message);
  }
}
