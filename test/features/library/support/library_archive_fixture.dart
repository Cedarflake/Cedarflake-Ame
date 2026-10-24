import "dart:async";

import "package:cedarflake_ame/features/library/application/library_organization_archive.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_archive.dart";

class ArchiveServiceFixture implements LibraryOrganizationArchives {
  final tasks = <ArchiveTaskFixture>[];

  @override
  LibraryArchiveTask createTask() {
    final task = ArchiveTaskFixture();
    tasks.add(task);
    return task;
  }
}

class ArchiveTaskFixture implements LibraryArchiveTask {
  final events = StreamController<LibraryArchiveEvent>(
    sync: true,
    // Keep subscription teardown in the widget test's asynchronous zone.
    onCancel: () async {},
  );
  String? kind;
  String? path;
  String? operationId;
  LibraryArchiveInspection? inspection;
  int cancellations = 0;
  int disposals = 0;

  @override
  Stream<LibraryArchiveEvent> export(String destination) {
    kind = "export";
    path = destination;
    return events.stream;
  }

  @override
  Stream<LibraryArchiveEvent> inspect(String path) {
    kind = "inspect";
    this.path = path;
    return events.stream;
  }

  @override
  Stream<LibraryArchiveEvent> restore(
    String operationId,
    String path,
    LibraryArchiveInspection inspection,
  ) {
    kind = "restore";
    this.path = path;
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

LibraryArchiveInspection archiveInspection({int revision = 10}) =>
    LibraryArchiveInspection(
      archiveId: "archive",
      checksum: "checksum-$revision",
      catalogRevision: BigInt.from(revision),
      organizationRevision: BigInt.from(revision),
      createdUnixMs: BigInt.from(1792830000000),
      albums: 3,
      groups: 10,
      favorites: 2,
      conflicts: 1,
      duplicateChoices: 4,
      matchedGroups: 7,
      unmatchedGroups: 3,
      unresolvedPreferredCopies: 1,
      hasContinuation: true,
    );
