import "../../../../src/rust/api/similar_image_review.dart" as api;
import "../../../../src/rust/domain/exact_duplicates.dart" as exact;
import "../../../../src/rust/domain/organization.dart" as organization;
import "../../../../src/rust/domain/similar_images.dart" as content;
import "../../../../src/rust/domain/similar_images/review.dart" as dto;
import "../../application/ports/library_similar_review.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_similar_review.dart";
import "library_asset_mapping.dart";
import "library_duplicate_failure.dart";

class RustLibrarySimilarReview implements LibrarySimilarReview {
  const RustLibrarySimilarReview();

  @override
  Future<LibrarySimilarGroups> loadGroups(
    String? after, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  }) async => _groups(
    await invokeDuplicateBridge(
      () => api.loadSimilarGroups(
        degree: _degree(degree),
        ignored: ignored,
        after: after,
      ),
    ),
  );

  @override
  Future<LibrarySimilarGroupRange> seekGroups(
    int index, {
    required LibrarySimilarityDegree degree,
    required bool ignored,
  }) async {
    final range = await invokeDuplicateBridge(
      () => api.seekSimilarGroups(
        degree: _degree(degree),
        ignored: ignored,
        index: BigInt.from(index),
      ),
    );
    return LibrarySimilarGroupRange(
      startIndex: range.startIndex.toInt(),
      page: _groups(range.page),
    );
  }

  @override
  Future<LibrarySimilarMembers> loadMembers(
    String groupId,
    LibrarySimilarityDegree degree,
    LibrarySimilarContent? after,
  ) async {
    final page = await invokeDuplicateBridge(
      () => api.loadSimilarMembers(
        groupId: groupId,
        degree: _degree(degree),
        after: after == null ? null : _content(after),
      ),
    );
    return LibrarySimilarMembers(
      members: page.members
          .map(
            (member) => LibrarySimilarMember(
              content: _readContent(member.content),
              asset: mapRustLibraryAsset(member.representative),
              availability: switch (member.availability) {
                exact.DuplicateLocationAvailability.local =>
                  LibraryDuplicateAvailability.local,
                exact.DuplicateLocationAvailability.cloudOnly =>
                  LibraryDuplicateAvailability.cloudOnly,
                exact.DuplicateLocationAvailability.offline =>
                  LibraryDuplicateAvailability.offline,
                exact.DuplicateLocationAvailability.unavailable =>
                  LibraryDuplicateAvailability.unavailable,
              },
              locationCount: member.locationCount.toInt(),
              distance: member.distance,
              preferred: member.preferred,
            ),
          )
          .toList(growable: false),
      total: page.totalMembers.toInt(),
      decision: _decision(page.decision),
      next: page.next == null ? null : _readContent(page.next!),
    );
  }

  @override
  Future<LibrarySimilarDecision> prefer(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    bool preferred,
  ) async => _decision(
    await invokeDuplicateBridge(
      () => api.setSimilarPreference(
        context: _context(context),
        selection: _selection(member),
        preferred: preferred,
      ),
    ),
  );

  @override
  Future<LibrarySimilarDecision> ignore(
    LibrarySimilarContext context,
    bool ignored,
  ) async => _decision(
    await invokeDuplicateBridge(
      () => api.setSimilarIgnored(context: _context(context), ignored: ignored),
    ),
  );

  @override
  Future<LibrarySimilarExclusionResult> exclude(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
    String exclusionId,
  ) async {
    final result = await invokeDuplicateBridge(
      () => api.excludeSimilarMember(
        context: _context(context),
        selection: _selection(member),
        exclusionId: exclusionId,
      ),
    );
    return LibrarySimilarExclusionResult(
      decision: _decision(result.decision),
      undo: LibrarySimilarExclusion(
        groupId: result.undo.groupId,
        content: _readContent(result.undo.content),
        id: result.undo.exclusionId,
      ),
    );
  }

  @override
  Future<LibrarySimilarUndoResult> undo(
    LibrarySimilarExclusion exclusion,
  ) async {
    final result = await invokeDuplicateBridge(
      () => api.undoSimilarExclusion(
        exclusion: dto.SimilarMemberExclusion(
          groupId: exclusion.groupId,
          content: _content(exclusion.content),
          exclusionId: exclusion.id,
        ),
      ),
    );
    return LibrarySimilarUndoResult(
      decision: _decision(result.decision),
      restored: result.restored,
    );
  }
}

dto.SimilarityDegree _degree(LibrarySimilarityDegree degree) =>
    switch (degree) {
      LibrarySimilarityDegree.strict => dto.SimilarityDegree.strict,
      LibrarySimilarityDegree.standard => dto.SimilarityDegree.standard,
      LibrarySimilarityDegree.broad => dto.SimilarityDegree.broad,
    };

content.SimilarContentKey _content(LibrarySimilarContent value) =>
    content.SimilarContentKey(
      scheme: value.scheme,
      digest: value.digest,
      byteLength: value.byteLength,
    );
LibrarySimilarContent _readContent(content.SimilarContentKey value) =>
    LibrarySimilarContent(
      scheme: value.scheme,
      digest: value.digest,
      byteLength: value.byteLength,
    );
LibrarySimilarDecision _decision(dto.SimilarGroupDecision value) =>
    LibrarySimilarDecision(revision: value.revision, ignored: value.ignored);
dto.SimilarReviewContext _context(LibrarySimilarContext value) =>
    dto.SimilarReviewContext(
      groupId: value.groupId,
      degree: _degree(value.degree),
      revision: value.revision,
    );
dto.SimilarMemberSelection _selection(LibrarySimilarMember member) =>
    dto.SimilarMemberSelection(
      content: _content(member.content),
      source: organization.OrganizationSelection(
        assetId: member.asset.assetId,
        locationId: member.asset.locationId,
        sourceGeneration: member.asset.sourceGeneration,
      ),
    );
LibrarySimilarGroups _groups(dto.SimilarReviewPage page) =>
    LibrarySimilarGroups(
      total: page.totalGroups.toInt(),
      next: page.nextGroupId,
      groups: page.groups
          .map(
            (group) => LibrarySimilarGroup(
              id: group.groupId,
              memberCount: group.memberCount.toInt(),
              representative: mapRustLibraryAsset(group.representative),
              decision: _decision(group.decision),
            ),
          )
          .toList(growable: false),
    );
