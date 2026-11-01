import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";

class ClassificationReviewFixture
    implements LibraryClassificationReviewService {
  final drafts = <ClassificationDraftFixture>[];
  final scopes = <LibraryClassificationScope>[];
  final undos = <String>[];
  Completer<LibraryClassificationDraft>? pendingOpen;
  Completer<LibraryClassificationReceipt>? pendingSave;
  LibraryClassificationSummary summary = const LibraryClassificationSummary(
    groups: 2,
    confirmable: 2,
    confirmed: 0,
  );

  @override
  Future<LibraryClassificationDraft> open(
    LibraryClassificationScope scope,
  ) async {
    scopes.add(scope);
    if (pendingOpen case final pending?) return pending.future;
    final draft = ClassificationDraftFixture(this);
    drafts.add(draft);
    return draft;
  }

  @override
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  ) async => [];

  @override
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  ) async => {};

  @override
  Future<LibraryClassificationReceipt> undo(
    String operationId,
    String originalOperationId,
  ) async {
    undos.add(originalOperationId);
    return LibraryClassificationReceipt(operationId, 2);
  }
}

class ClassificationDraftFixture implements LibraryClassificationDraft {
  ClassificationDraftFixture(this.service);
  final ClassificationReviewFixture service;
  final chunks = <List<String>>[];
  final targets = <LibraryOrganizationTarget>[];
  final commands = <LibraryClassificationCommand>[];
  final operationIds = <String>[];
  int preparations = 0;
  int closes = 0;
  bool _closed = false;

  @override
  Future<void> append(List<String> ids) async => chunks.add(ids);
  @override
  Future<void> appendTargets(List<LibraryOrganizationTarget> values) async =>
      targets.addAll(values);
  @override
  Future<LibraryClassificationSummary> prepare() async {
    preparations++;
    return service.summary;
  }

  @override
  Future<LibraryClassificationReceipt> apply(
    String operationId,
    LibraryClassificationCommand command,
  ) async {
    commands.add(command);
    operationIds.add(operationId);
    return service.pendingSave?.future ??
        LibraryClassificationReceipt(operationId, 2);
  }

  @override
  Future<void> close() async {
    if (!_closed) {
      _closed = true;
      closes++;
    }
  }
}

LibraryClassificationScope classificationScope({
  Iterable<String> ids = const ["a", "b"],
  bool all = false,
}) => LibraryClassificationScope(
  query: const LibraryGalleryQuery(
    collection: LibraryGalleryCollection.smartAlbum(),
  ),
  catalogRevision: BigInt.one,
  allMatching: all,
  assetIds: ids,
);
