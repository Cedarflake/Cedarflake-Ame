import "../../domain/library_classification_configuration.dart";

abstract interface class LibraryClassificationConfigurationService {
  Future<LibraryClassificationSettings> load();
  Future<LibraryClassificationSettings> loadNavigation();
  Future<List<LibraryClassificationCategory>> initialCategories();
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  });
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  });
}
