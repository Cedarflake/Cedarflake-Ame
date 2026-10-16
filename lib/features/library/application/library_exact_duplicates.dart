import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/exact_duplicates.dart" as rust_api;
import "../../../src/rust/domain/exact_duplicates.dart" as rust_domain;
import "../domain/library_exact_duplicates.dart";

abstract interface class LibraryExactDuplicates {
  Stream<LibraryExactAnalysisProgress> analyze(String runId);
  Future<void> cancel(String runId);
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  );
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  );
}

class RustLibraryExactDuplicates implements LibraryExactDuplicates {
  const RustLibraryExactDuplicates();

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    return rust_api.analyzeExactDuplicates(runId: runId).map((progress) {
      return LibraryExactAnalysisProgress(
        runId: progress.runId,
        phase: switch (progress.phase) {
          rust_domain.ExactAnalysisPhase.running =>
            LibraryExactAnalysisPhase.running,
          rust_domain.ExactAnalysisPhase.completed =>
            LibraryExactAnalysisPhase.completed,
          rust_domain.ExactAnalysisPhase.cancelled =>
            LibraryExactAnalysisPhase.cancelled,
          rust_domain.ExactAnalysisPhase.failed =>
            LibraryExactAnalysisPhase.failed,
        },
        checkedFiles: progress.checkedFiles.toInt(),
        fingerprintedFiles: progress.fingerprintedFiles.toInt(),
        skippedFiles: progress.skippedFiles.toInt(),
        currentPath: progress.currentPath,
      );
    });
  }

  @override
  Future<void> cancel(String runId) =>
      rust_api.cancelExactDuplicateAnalysis(runId: runId);

  @override
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  ) async {
    final page = await rust_api.loadExactDuplicateGroups(
      after: after == null
          ? null
          : rust_domain.ExactDuplicateGroupCursor(
              digest: after.digest,
              byteLength: BigInt.from(after.byteLength),
            ),
    );
    return LibraryDuplicateGroupPage(
      groups: page.groups
          .map(
            (group) => LibraryDuplicateGroup(
              digest: group.digest,
              byteLength: group.byteLength.toInt(),
              locationCount: group.locationCount.toInt(),
              physicalFileCount: group.physicalFileCount.toInt(),
              representativeLocationId: group.representativeLocationId,
              representativePath: group.representativePath,
            ),
          )
          .toList(growable: false),
      next: page.next == null
          ? null
          : LibraryDuplicateGroupCursor(
              page.next!.digest,
              page.next!.byteLength.toInt(),
            ),
    );
  }

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async {
    final page = await rust_api.loadExactDuplicateLocations(
      digest: group.digest,
      byteLength: BigInt.from(group.byteLength),
      afterLocationId: after,
    );
    return LibraryDuplicateLocationPage(
      locations: page.locations
          .map(
            (location) => LibraryDuplicateLocation(
              locationId: location.locationId,
              displayPath: location.displayPath,
              availability: switch (location.availability) {
                rust_domain.DuplicateLocationAvailability.local =>
                  LibraryDuplicateAvailability.local,
                rust_domain.DuplicateLocationAvailability.cloudOnly =>
                  LibraryDuplicateAvailability.cloudOnly,
                rust_domain.DuplicateLocationAvailability.offline =>
                  LibraryDuplicateAvailability.offline,
                rust_domain.DuplicateLocationAvailability.unavailable =>
                  LibraryDuplicateAvailability.unavailable,
              },
            ),
          )
          .toList(growable: false),
      nextLocationId: page.nextLocationId,
    );
  }
}

final libraryExactDuplicatesProvider = Provider<LibraryExactDuplicates>(
  (ref) => const RustLibraryExactDuplicates(),
);
