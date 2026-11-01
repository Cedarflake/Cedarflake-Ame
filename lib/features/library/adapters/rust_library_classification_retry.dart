import "../../../src/rust/api/classification_retry.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/classification/work.dart" as work;
import "../../../src/rust/domain/organization.dart" as organization;
import "../application/library_classification_retry.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";

class RustLibraryClassificationRetry implements LibraryClassificationRetry {
  const RustLibraryClassificationRetry();

  @override
  Future<LibraryClassificationRetryOutcome> retry(
    LibraryOrganizationTarget target,
    BigInt expectedDispatchRevision,
  ) async {
    try {
      final result = await api.retryFailedClassification(
        expectedDispatchRevision: expectedDispatchRevision,
        target: organization.OrganizationSelection(
          assetId: target.assetId,
          locationId: target.locationId,
          sourceGeneration: target.sourceGeneration,
        ),
      );
      return switch (result) {
        work.ClassificationRetryOutcome.unchanged =>
          LibraryClassificationRetryOutcome.unchanged,
        work.ClassificationRetryOutcome.queued =>
          LibraryClassificationRetryOutcome.queued,
        work.ClassificationRetryOutcome.queuedWhilePaused =>
          LibraryClassificationRetryOutcome.queuedWhilePaused,
      };
    } on rust.ScanError catch (error) {
      throw LibraryClassificationFailure(error.code, error.message);
    }
  }
}
