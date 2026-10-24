import "dart:async";

import "package:cedarflake_ame/features/library/application/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";

class OrganizationBatchesFixture implements LibraryOrganizationBatches {
  final selections = <LibraryOrganizationSelection>[];
  final batches = <OrganizationBatchFixture>[];
  Future<LibraryOrganizationBatch> Function()? opening;
  OrganizationBatchFixture Function() create = OrganizationBatchFixture.new;

  @override
  Future<LibraryOrganizationBatch> open(
    LibraryOrganizationSelection selection,
  ) async {
    selections.add(selection);
    if (opening case final open?) return open();
    final batch = create();
    batches.add(batch);
    return batch;
  }
}

class OrganizationBatchFixture implements LibraryOrganizationBatch {
  final chunks = <List<String>>[];
  final writes = <({String id, LibraryBatchAction action})>[];
  LibraryBatchSummary summary = const LibraryBatchSummary(
    groups: 3,
    favorites: 1,
    assigned: 2,
    conflicts: 0,
  );
  Completer<LibraryBatchSummary>? preparing;
  Future<void> Function()? refreshing;
  Future<LibraryOrganizationReceipt> Function(String)? saving;
  void Function()? onClose;
  int preparations = 0;
  int refreshes = 0;
  int closes = 0;
  LibraryBatchPhase phase = LibraryBatchPhase.collecting;

  @override
  Future<void> append(List<String> assetIds) async =>
      chunks.add(List.of(assetIds));
  @override
  Future<LibraryBatchSummary> prepare() async {
    preparations++;
    phase = LibraryBatchPhase.preparing;
    final result = await (preparing?.future ?? Future.value(summary));
    phase = LibraryBatchPhase.ready;
    return result;
  }

  @override
  Future<LibraryBatchSummary> refresh() async {
    refreshes++;
    await refreshing?.call();
    return summary;
  }

  @override
  Future<LibraryOrganizationReceipt> apply(
    String operationId,
    LibraryBatchAction action,
  ) async {
    writes.add((id: operationId, action: action));
    phase = LibraryBatchPhase.saving;
    final result =
        await (saving?.call(operationId) ?? Future.value(receipt(operationId)));
    phase = LibraryBatchPhase.completed;
    return result;
  }

  LibraryOrganizationReceipt receipt(String id) => LibraryOrganizationReceipt(
    operationId: id,
    revision: BigInt.two,
    changedGroups: summary.groups,
  );
  @override
  Future<LibraryBatchProgress> progress() async =>
      LibraryBatchProgress(phase, 0, summary.groups);
  @override
  Future<void> close() async {
    closes++;
    onClose?.call();
  }
}
