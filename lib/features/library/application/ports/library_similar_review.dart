import "../../domain/library_similar_review.dart";

abstract interface class LibrarySimilarReview {
  Future<LibrarySimilarGroups> loadGroups(
    String? after, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  });
  Future<LibrarySimilarGroupRange> seekGroups(
    int index, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  });
  Future<LibrarySimilarMembers> loadMembers(
    String groupId,
    LibrarySimilarityDegree degree,
    LibrarySimilarContent? after,
  );
  Future<LibrarySimilarDecision> prefer(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    bool preferred,
  );
  Future<LibrarySimilarDecision> ignore(
    LibrarySimilarContext context,
    bool ignored,
  );
  Future<LibrarySimilarExclusionResult> exclude(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    String exclusionId,
  );
  Future<LibrarySimilarUndoResult> undo(LibrarySimilarExclusion exclusion);
}
