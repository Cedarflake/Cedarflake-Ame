import "dart:async";

import "package:cedarflake_ame/features/library/application/library_organization_cleanup.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_cleanup.dart";

class CleanupServiceFixture implements LibraryOrganizationCleanup {
  final tasks = <CleanupTaskFixture>[];
  @override
  LibraryCleanupTask createTask() {
    final task = CleanupTaskFixture();
    tasks.add(task);
    return task;
  }
}

class CleanupTaskFixture implements LibraryCleanupTask {
  final events = StreamController<LibraryCleanupEvent>(
    sync: true,
    onCancel: () async {},
  );
  int? cutoffUnixMs;
  String? operationId;
  LibraryCleanupInspection? inspection;
  int cancellations = 0;
  int disposals = 0;
  @override
  Stream<LibraryCleanupEvent> inspect(int? cutoffUnixMs) {
    this.cutoffUnixMs = cutoffUnixMs;
    return events.stream;
  }

  @override
  Stream<LibraryCleanupEvent> apply(
    String operationId,
    LibraryCleanupInspection inspection,
  ) {
    this.operationId = operationId;
    this.inspection = inspection;
    return events.stream;
  }

  @override
  void cancel() => cancellations++;
  @override
  void dispose() {
    disposals++;
    unawaited(events.close());
  }
}

LibraryCleanupInspection cleanupInspection({
  int groups = 4,
  int classificationGroups = 0,
}) => LibraryCleanupInspection(
  cutoffUnixMs: 1790000000000,
  inspectedUnixMs: 1792830000000,
  catalogRevision: BigInt.from(10),
  organizationRevision: BigInt.from(10),
  selectionDigest: "a" * 64,
  groups: groups,
  albumGroups: groups,
  favoriteGroups: groups,
  progressGroups: groups,
  classificationGroups: classificationGroups,
  duplicateChoices: 0,
  oldestUnassociatedUnixMs: groups > 0 ? 1780000000000 : null,
  newestUnassociatedUnixMs: groups > 0 ? 1781000000000 : null,
);
