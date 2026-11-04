import "library_exact_duplicates.dart";
import "library_models.dart";

enum LibrarySimilarityDegree { strict, standard, broad }

class LibrarySimilarContent {
  const LibrarySimilarContent({
    required this.scheme,
    required this.digest,
    required this.byteLength,
  });
  final String scheme;
  final String digest;
  final BigInt byteLength;

  @override
  bool operator ==(Object other) =>
      other is LibrarySimilarContent &&
      scheme == other.scheme &&
      digest == other.digest &&
      byteLength == other.byteLength;

  @override
  int get hashCode => Object.hash(scheme, digest, byteLength);
}

class LibrarySimilarDecision {
  const LibrarySimilarDecision({
    required this.revision,
    this.preferred,
    this.ignored = false,
  });
  final BigInt revision;
  final LibrarySimilarContent? preferred;
  final bool ignored;
}

class LibrarySimilarGroup {
  const LibrarySimilarGroup({
    required this.id,
    required this.memberCount,
    required this.representative,
    required this.decision,
  });
  final String id;
  final int memberCount;
  final LibraryAsset representative;
  final LibrarySimilarDecision decision;
}

class LibrarySimilarGroups {
  const LibrarySimilarGroups({
    required this.groups,
    required this.total,
    this.next,
  });
  final List<LibrarySimilarGroup> groups;
  final int total;
  final String? next;
}

class LibrarySimilarGroupRange {
  const LibrarySimilarGroupRange({
    required this.startIndex,
    required this.page,
  });
  final int startIndex;
  final LibrarySimilarGroups page;
}

class LibrarySimilarMember {
  const LibrarySimilarMember({
    required this.content,
    required this.asset,
    required this.availability,
    required this.locationCount,
    required this.distance,
  });
  final LibrarySimilarContent content;
  final LibraryAsset asset;
  final LibraryDuplicateAvailability availability;
  final int locationCount;
  final int distance;
}

class LibrarySimilarMembers {
  const LibrarySimilarMembers({
    required this.members,
    required this.total,
    required this.decision,
    this.next,
  });
  final List<LibrarySimilarMember> members;
  final int total;
  final LibrarySimilarDecision decision;
  final LibrarySimilarContent? next;
}

class LibrarySimilarContext {
  const LibrarySimilarContext({
    required this.groupId,
    required this.degree,
    required this.revision,
  });
  final String groupId;
  final LibrarySimilarityDegree degree;
  final BigInt revision;
}

class LibrarySimilarExclusion {
  const LibrarySimilarExclusion({
    required this.groupId,
    required this.content,
    required this.id,
  });
  final String groupId;
  final LibrarySimilarContent content;
  final String id;
}

class LibrarySimilarExclusionResult {
  const LibrarySimilarExclusionResult({
    required this.decision,
    required this.undo,
  });
  final LibrarySimilarDecision decision;
  final LibrarySimilarExclusion undo;
}

class LibrarySimilarUndoResult {
  const LibrarySimilarUndoResult({
    required this.decision,
    required this.restored,
  });
  final LibrarySimilarDecision decision;
  final bool restored;
}
