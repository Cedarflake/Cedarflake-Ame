import "../../domain/library_classification_configuration.dart";
import "../../domain/library_classification_term.dart";

abstract interface class LibraryClassificationTermsService {
  Future<List<LibraryClassificationTerm>> load();
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationCategory? expected,
    required LibraryClassificationCategory replacement,
  });
  Future<LibraryClassificationSettings> remove({
    required String operationId,
    required LibraryClassificationCategory expected,
  });
  Future<LibraryClassificationSettings> reset({
    required String operationId,
    required LibraryClassificationCategory expected,
  });
  Future<LibraryClassificationSettings> resetAll({
    required String operationId,
    required List<LibraryClassificationCategory> expected,
  });
}
