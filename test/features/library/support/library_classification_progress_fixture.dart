import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";

class ClassificationProgressFixture implements LibraryClassificationProgress {
  final scopes = <LibraryGalleryQuery>[];
  final positions = <LibraryClassificationPosition>[];
  final targets = <LibraryOrganizationTarget>[];
  final ended = <String>[];
  Completer<LibraryClassificationPosition>? pendingBegin;
  Completer<void>? pendingCheckpoint;
  Completer<LibraryClassificationContinuation>? pendingContinuation;
  LibraryClassificationContinuation saved =
      const LibraryClassificationContinuation(
        LibraryClassificationContinuationStatus.noActivity,
      );

  @override
  Future<LibraryClassificationPosition> begin(LibraryGalleryQuery query) async {
    scopes.add(query);
    return pendingBegin?.future ??
        LibraryClassificationPosition("review", BigInt.zero);
  }

  @override
  Future<void> checkpoint(
    LibraryClassificationPosition position,
    LibraryOrganizationTarget target,
  ) async {
    positions.add(position);
    targets.add(target);
    await pendingCheckpoint?.future;
  }

  @override
  Future<void> end(String sessionId) async => ended.add(sessionId);
  @override
  Future<LibraryClassificationContinuation> continuation() async =>
      pendingContinuation?.future ?? saved;
}
