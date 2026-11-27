import "library_classification_configuration.dart";

class LibraryClassificationTerm {
  const LibraryClassificationTerm({
    required this.category,
    required this.isCustom,
    this.defaultCategory,
  });

  final LibraryClassificationCategory category;
  final bool isCustom;
  final LibraryClassificationCategory? defaultCategory;
}

class LibraryClassificationTermChange {
  const LibraryClassificationTermChange(this.id, this.term);

  final String id;
  final LibraryClassificationTerm? term;
}
