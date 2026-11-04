import "../../../../src/rust/api/classification_review.dart" as api;
import "../../../../src/rust/domain/classification_selection.dart" as dto;
import "../../../../src/rust/domain/organization.dart" as organization;
import "../../application/ports/library_classification_review.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_organization.dart";
import "library_classification_failure.dart";

class RustLibraryClassificationReview
    implements LibraryClassificationReviewService {
  const RustLibraryClassificationReview();

  @override
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  ) => invokeClassificationBridge(() async {
    final values = await api.loadClassificationItems(
      selections: [
        for (final target in targets)
          organization.OrganizationSelection(
            assetId: target.assetId,
            locationId: target.locationId,
            sourceGeneration: target.sourceGeneration,
          ),
      ],
    );
    return [for (final value in values) _item(value)];
  });

  @override
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  ) => invokeClassificationBridge(() async {
    final values = await api.loadClassificationAnnotations(
      catalogRevision: revision,
      selections: [
        for (final target in targets)
          organization.OrganizationSelection(
            assetId: target.assetId,
            locationId: target.locationId,
            sourceGeneration: target.sourceGeneration,
          ),
      ],
    );
    return {
      for (final value in values) value.locationId: _reason(value.reason),
    };
  });
}

LibraryClassificationItem _item(dto.ClassificationItemState value) =>
    LibraryClassificationItem(
      assetId: value.assetId,
      categoryId: value.categoryId,
      reviewReason: value.reviewReason == null
          ? null
          : _reason(value.reviewReason!),
      review: switch (value.review) {
        dto.ClassificationItemReview.unclassified =>
          LibraryClassificationReview.unclassified,
        dto.ClassificationItemReview.suggested =>
          LibraryClassificationReview.suggested,
        dto.ClassificationItemReview.needsReview =>
          LibraryClassificationReview.needsReview,
        dto.ClassificationItemReview.deferred_ =>
          LibraryClassificationReview.deferred,
        dto.ClassificationItemReview.confirmed =>
          LibraryClassificationReview.confirmed,
        dto.ClassificationItemReview.conflict =>
          LibraryClassificationReview.conflict,
      },
    );

LibraryClassificationReviewReason _reason(
  dto.ClassificationReviewReason value,
) => switch (value) {
  dto.ClassificationReviewReason.uncertain =>
    LibraryClassificationReviewReason.uncertain,
  dto.ClassificationReviewReason.uncalibrated =>
    LibraryClassificationReviewReason.uncalibrated,
  dto.ClassificationReviewReason.sampling =>
    LibraryClassificationReviewReason.sampling,
  dto.ClassificationReviewReason.deferred_ =>
    LibraryClassificationReviewReason.deferred,
  dto.ClassificationReviewReason.conflict =>
    LibraryClassificationReviewReason.conflict,
  dto.ClassificationReviewReason.analysisFailed =>
    LibraryClassificationReviewReason.analysisFailed,
};
