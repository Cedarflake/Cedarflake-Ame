import "../../../../src/rust/api/classification_terms.dart" as native;
import "../../../../src/rust/domain/classification_configuration.dart"
    as contract;
import "../../application/ports/library_classification_terms.dart";
import "../../domain/library_classification_configuration.dart";
import "../../domain/library_classification_term.dart";
import "library_classification_failure.dart";
import "rust_library_classification_configuration.dart";

class RustLibraryClassificationTerms
    implements LibraryClassificationTermsService {
  const RustLibraryClassificationTerms();

  @override
  Future<List<LibraryClassificationTerm>> load() => invokeClassificationBridge(
    () async => List.unmodifiable(
      (await native.loadClassificationTerms()).map((term) {
        final category = term.category;
        final defaults = term.defaultCategory;
        return LibraryClassificationTerm(
          category: LibraryClassificationCategory(
            id: category.id,
            label: category.label,
            descriptions: List.unmodifiable(category.descriptions),
            defaultLabel: defaults?.label,
          ),
          defaultCategory: defaults == null
              ? null
              : classificationInputFromRust(defaults),
          isCustom: term.isCustom,
          isConfigured: term.isConfigured,
        );
      }),
    ),
  );

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationCategory? expected,
    required LibraryClassificationCategory replacement,
  }) => invokeClassificationBridge(
    () async => classificationSettingsFromRust(
      await native.saveClassificationTerm(
        operationId: operationId,
        expected: expected == null ? null : _input(expected),
        replacement: _input(replacement),
      ),
    ),
  );

  @override
  Future<LibraryClassificationSettings> reset({
    required String operationId,
    required LibraryClassificationCategory expected,
  }) => invokeClassificationBridge(
    () async => classificationSettingsFromRust(
      await native.resetClassificationTerm(
        operationId: operationId,
        expected: _input(expected),
      ),
    ),
  );

  @override
  Future<LibraryClassificationSettings> remove({
    required String operationId,
    required LibraryClassificationCategory expected,
  }) => invokeClassificationBridge(
    () async => classificationSettingsFromRust(
      await native.removeClassificationTerm(
        operationId: operationId,
        expected: _input(expected),
      ),
    ),
  );

  contract.ClassificationCategoryInput _input(
    LibraryClassificationCategory category,
  ) => contract.ClassificationCategoryInput(
    id: category.id,
    label: category.label,
    descriptions: category.descriptions,
  );
}
