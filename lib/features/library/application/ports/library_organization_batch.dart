import "../../domain/library_organization.dart";
import "../../domain/library_organization_batch.dart";

abstract interface class LibraryOrganizationBatches {
  Future<LibraryOrganizationBatch> open(LibraryOrganizationSelection selection);
}

abstract interface class LibraryOrganizationBatch {
  Future<void> append(List<String> assetIds);
  Future<LibraryBatchSummary> prepare();
  Future<LibraryBatchSummary> refresh();
  Future<LibraryOrganizationReceipt> apply(
    String operationId,
    LibraryBatchAction action,
  );
  Future<LibraryBatchProgress> progress();
  Future<void> close();
}
