typedef LibrarySemanticSource = ({
  String assetId,
  String locationId,
  BigInt sourceGeneration,
});

sealed class LibrarySemanticInput {
  const LibrarySemanticInput();
}

class LibrarySemanticDescription extends LibrarySemanticInput {
  const LibrarySemanticDescription(this.text);
  final String text;
}

class LibrarySemanticReference extends LibrarySemanticInput {
  const LibrarySemanticReference(this.source, this.name);
  final LibrarySemanticSource source;
  final String name;
}

class LibrarySemanticResults {
  LibrarySemanticResults({
    required this.id,
    required this.catalogRevision,
    required this.featureGeneration,
    required this.eligibleItems,
    required this.indexedItems,
    required this.storageFull,
    required List<LibrarySemanticSource> items,
  }) : items = List.unmodifiable(items);

  final String id;
  final BigInt catalogRevision;
  final BigInt featureGeneration;
  final int eligibleItems;
  final int indexedItems;
  final bool storageFull;
  final List<LibrarySemanticSource> items;

  @override
  int get hashCode => id.hashCode;
  @override
  bool operator ==(Object other) =>
      other is LibrarySemanticResults && other.id == id;
}

class LibrarySemanticIndexStatus {
  const LibrarySemanticIndexStatus({
    required this.indexing,
    required this.featureGeneration,
    required this.budgetReached,
    this.blocked = false,
    this.catalogRevision,
    this.issueCode,
  });
  final bool indexing;
  final bool blocked;
  final BigInt? catalogRevision;
  final BigInt featureGeneration;
  final bool budgetReached;
  final String? issueCode;
}

class LibrarySemanticFailure implements Exception {
  const LibrarySemanticFailure(this.code);
  final String code;

  @override
  String toString() => code;
}

class LibrarySemanticIndexFailure extends LibrarySemanticFailure {
  const LibrarySemanticIndexFailure(this.causeCode)
    : super("semantic_index_blocked");
  final String? causeCode;

  @override
  String toString() => "$code: $causeCode";
}
