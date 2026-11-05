import "dart:async";

import "package:cedarflake_ame/features/library/application/ports/library_similar_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_review.dart";

import "duplicate_review_fixtures.dart";

LibrarySimilarMember similarMember(String id, {bool preferred = false}) =>
    LibrarySimilarMember(
      content: LibrarySimilarContent(
        scheme: "fixture-exact-v1",
        digest: id,
        byteLength: BigInt.from(1024),
      ),
      asset: reviewAsset(id),
      availability: LibraryDuplicateAvailability.local,
      locationCount: id == "one" ? 3 : 1,
      distance: id == "one" ? 0 : 3,
      preferred: preferred,
    );

class SimilarReviewFixture implements LibrarySimilarReview {
  final members = [similarMember("one"), similarMember("two")];
  LibrarySimilarDecision decision = LibrarySimilarDecision(
    revision: BigInt.zero,
  );
  final queries = <({LibrarySimilarityDegree degree, bool ignored})>[];
  final memberQueries = <({String groupId, LibrarySimilarityDegree degree})>[];
  Completer<LibrarySimilarGroups>? pendingGroups;
  Completer<LibrarySimilarMembers>? pendingMembers;
  Completer<LibrarySimilarDecision>? pendingWrite;
  LibrarySimilarContext? lastContext;
  LibrarySimilarMember? excluded;
  LibrarySimilarExclusion? lastUndo;
  bool failWrite = false;
  bool restoreUndo = true;
  int writes = 0;

  LibrarySimilarGroup group([String id = "similar-one"]) => LibrarySimilarGroup(
    id: id,
    memberCount: members.length,
    representative: reviewAsset("one"),
    decision: decision,
  );

  LibrarySimilarMembers get memberPage => LibrarySimilarMembers(
    members: List.of(members),
    total: members.length,
    decision: decision,
  );

  @override
  Future<LibrarySimilarGroups> loadGroups(
    String? after, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  }) async {
    queries.add((degree: degree, ignored: ignored));
    if (pendingGroups case final pending?) return pending.future;
    final groups = members.length > 1 && decision.ignored == ignored
        ? [group()]
        : <LibrarySimilarGroup>[];
    return LibrarySimilarGroups(groups: groups, total: groups.length);
  }

  @override
  Future<LibrarySimilarGroupRange> seekGroups(
    int index, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  }) async => LibrarySimilarGroupRange(
    startIndex: 0,
    page: await loadGroups(null, degree: degree, ignored: ignored),
  );

  @override
  Future<LibrarySimilarMembers> loadMembers(
    String groupId,
    LibrarySimilarityDegree degree,
    LibrarySimilarContent? after,
  ) async {
    memberQueries.add((groupId: groupId, degree: degree));
    if (pendingMembers case final pending?) return pending.future;
    return memberPage;
  }

  void _write(LibrarySimilarContext context) {
    lastContext = context;
    writes++;
    if (failWrite) {
      throw const LibraryDuplicateFailure(
        code: "similarity_review_stale",
        message: "Fixture revision changed",
      );
    }
  }

  @override
  Future<LibrarySimilarDecision> prefer(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    bool preferred,
  ) async {
    _write(context);
    if (pendingWrite case final pending?) return pending.future;
    final index = members.indexWhere(
      (value) => value.content == member.content,
    );
    members[index] = similarMember(
      member.asset.locationId,
      preferred: preferred,
    );
    return decision = LibrarySimilarDecision(
      revision: decision.revision + BigInt.one,
      ignored: decision.ignored,
    );
  }

  @override
  Future<LibrarySimilarDecision> ignore(
    LibrarySimilarContext context,
    bool ignored,
  ) async {
    _write(context);
    return decision = LibrarySimilarDecision(
      revision: decision.revision + BigInt.one,
      ignored: ignored,
    );
  }

  @override
  Future<LibrarySimilarExclusionResult> exclude(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    String exclusionId,
  ) async {
    _write(context);
    excluded = members.firstWhere((value) => value.content == member.content);
    members.removeWhere((value) => value.content == member.content);
    decision = LibrarySimilarDecision(
      revision: decision.revision + BigInt.one,
      ignored: decision.ignored,
    );
    return LibrarySimilarExclusionResult(
      decision: decision,
      undo: LibrarySimilarExclusion(
        groupId: context.groupId,
        content: member.content,
        id: exclusionId,
      ),
    );
  }

  @override
  Future<LibrarySimilarUndoResult> undo(
    LibrarySimilarExclusion exclusion,
  ) async {
    lastUndo = exclusion;
    if (restoreUndo && excluded != null) members.add(excluded!);
    return LibrarySimilarUndoResult(decision: decision, restored: restoreUndo);
  }
}
