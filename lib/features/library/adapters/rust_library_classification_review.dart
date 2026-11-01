import "../../../src/rust/api/classification_review.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/classification_selection.dart" as dto;
import "../../../src/rust/domain/organization.dart" as organization;
import "../application/library_catalog.dart";
import "../application/library_classification_review.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";

class RustLibraryClassificationReview
    implements LibraryClassificationReviewService {
  const RustLibraryClassificationReview();

  @override
  Future<LibraryClassificationDraft> open(LibraryClassificationScope scope) =>
      _call(
        () async => _Draft(
          await api.openClassificationSelection(
            query: mapLibraryGalleryQueryToRust(scope.query),
            catalogRevision: scope.catalogRevision,
            allMatching: scope.allMatching,
          ),
        ),
      );

  @override
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  ) => _call(() async {
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
  Future<LibraryClassificationReceipt> undo(
    String operationId,
    String originalOperationId,
  ) => _call(() async {
    final receipt = await api.undoClassificationChange(
      operationId: operationId,
      originalOperationId: originalOperationId,
    );
    return LibraryClassificationReceipt(
      receipt.operationId,
      receipt.changedGroups.toInt(),
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

class _Draft implements LibraryClassificationDraft {
  _Draft(this.native);
  final api.ClassificationSelectionDraft native;
  Future<void>? _closing;

  @override
  Future<void> appendTargets(List<LibraryOrganizationTarget> targets) => _call(
    () => native.appendTargets(
      targets: [
        for (final target in targets)
          organization.OrganizationSelection(
            assetId: target.assetId,
            locationId: target.locationId,
            sourceGeneration: target.sourceGeneration,
          ),
      ],
    ),
  );

  @override
  Future<void> append(List<String> ids) =>
      _call(() => native.appendIds(assetIds: ids));
  @override
  Future<LibraryClassificationSummary> prepare() => _call(() async {
    final summary = await native.prepare();
    return LibraryClassificationSummary(
      groups: summary.groupCount.toInt(),
      confirmable: summary.confirmableCount.toInt(),
      confirmed: summary.confirmedCount.toInt(),
      commonCategoryId: summary.commonCategoryId,
      singleItem: summary.singleItem == null
          ? null
          : _item(summary.singleItem!),
    );
  });
  @override
  Future<LibraryClassificationReceipt> apply(
    String operationId,
    LibraryClassificationCommand command,
  ) => _call(() async {
    final receipt = await switch (command) {
      LibraryConfirmClassification() => native.confirm(
        operationId: operationId,
      ),
      LibraryCorrectClassification(:final categoryId) => native.correct(
        operationId: operationId,
        categoryId: categoryId,
      ),
      LibraryDeferClassification() => native.defer(operationId: operationId),
    };
    return LibraryClassificationReceipt(
      receipt.operationId,
      receipt.changedGroups.toInt(),
    );
  });
  @override
  Future<void> close() => _closing ??= _close();
  Future<void> _close() async {
    try {
      await _call(native.close);
    } finally {
      native.dispose();
    }
  }
}

LibraryClassificationItem _item(dto.ClassificationItemState value) =>
    LibraryClassificationItem(
      assetId: value.assetId,
      categoryId: value.categoryId,
      canConfirm: value.canConfirm,
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
