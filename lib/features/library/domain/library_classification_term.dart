import "library_classification_configuration.dart";

class LibraryClassificationTerm {
  const LibraryClassificationTerm({
    required this.category,
    required this.isCustom,
    this.isConfigured = true,
    this.defaultCategory,
  });

  final LibraryClassificationCategory category;
  final bool isCustom;
  final bool isConfigured;
  final LibraryClassificationCategory? defaultCategory;

  bool get canRestoreDefaults {
    final defaults = defaultCategory;
    if (isCustom || !isConfigured || defaults == null) return false;
    return category.label != defaults.label ||
        category.descriptions.length != defaults.descriptions.length ||
        !category.descriptions.toSet().containsAll(defaults.descriptions);
  }
}

class LibraryClassificationTermChange {
  const LibraryClassificationTermChange(this.id, this.term);

  final String id;
  final LibraryClassificationTerm? term;
}
