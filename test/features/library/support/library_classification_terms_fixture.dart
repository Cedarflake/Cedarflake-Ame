import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_terms.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification_terms.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_term.dart";

const starterCategory = LibraryClassificationCategory(
  id: "photo",
  label: "Photography",
  descriptions: ["a photograph"],
  defaultLabel: "Photography",
);
const starterTerm = LibraryClassificationTerm(
  category: starterCategory,
  isCustom: false,
  defaultCategory: starterCategory,
);

ClassificationTermsFixture modifiedTermsFixture() =>
    ClassificationTermsFixture()
      ..terms = [
        const LibraryClassificationTerm(
          category: LibraryClassificationCategory(
            id: "photo",
            label: "My photographs",
            descriptions: ["a photograph"],
          ),
          isCustom: false,
          defaultCategory: starterCategory,
        ),
        const LibraryClassificationTerm(
          category: LibraryClassificationCategory(
            id: "art",
            label: "Art",
            descriptions: ["an oil painting"],
          ),
          isCustom: false,
          defaultCategory: LibraryClassificationCategory(
            id: "art",
            label: "Art",
            descriptions: ["a painting"],
          ),
        ),
        const LibraryClassificationTerm(
          category: LibraryClassificationCategory(
            id: "landscape",
            label: "Landscape",
            descriptions: ["a landscape"],
          ),
          isCustom: false,
          isConfigured: false,
        ),
        const LibraryClassificationTerm(
          category: LibraryClassificationCategory(
            id: "custom-cats",
            label: "Cats",
            descriptions: ["a cat"],
          ),
          isCustom: true,
        ),
      ];

class ClassificationTermsFixture implements LibraryClassificationTermsService {
  List<LibraryClassificationTerm> terms = [starterTerm];
  int retired = 0;
  int changes = 0;
  int loads = 0;
  int saves = 0;
  int resets = 0;
  List<String> lastResetIds = [];
  LibraryClassificationCategory? lastExpected;
  Completer<void>? saveGate;
  bool reject = false;
  late final editor = LibraryClassificationTerms(
    this,
    changeConfiguration: (action) {
      retired++;
      return action();
    },
    onChanged: () => changes++,
  );

  LibraryClassificationSettings get settings => LibraryClassificationSettings(
    revision: BigInt.one,
    dispatchRevision: BigInt.one,
    mode: LibraryClassificationMode.automatic,
    categories: terms
        .where((term) => term.isConfigured)
        .map((term) => term.category)
        .toList(),
    runId: "run",
    installationSelected: true,
  );

  @override
  Future<List<LibraryClassificationTerm>> load() async {
    loads++;
    return List.of(terms);
  }

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationCategory? expected,
    required LibraryClassificationCategory replacement,
  }) async {
    saves++;
    lastExpected = expected;
    await saveGate?.future;
    if (reject) {
      throw const LibraryClassificationFailure(
        "classification_term_changed",
        "Changed",
      );
    }
    final index = terms.indexWhere(
      (term) => term.category.id == replacement.id,
    );
    final original = index < 0 ? null : terms[index];
    final updated = LibraryClassificationTerm(
      category: replacement,
      isCustom: original?.isCustom ?? true,
      defaultCategory: original?.defaultCategory,
    );
    if (index < 0) {
      terms.add(updated);
    } else {
      terms[index] = updated;
    }
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> remove({
    required String operationId,
    required LibraryClassificationCategory expected,
  }) async {
    terms.removeWhere((term) => term.category.id == expected.id);
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> reset({
    required String operationId,
    required LibraryClassificationCategory expected,
  }) async {
    final index = terms.indexWhere((term) => term.category.id == expected.id);
    terms[index] = starterTerm;
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> resetAll({
    required String operationId,
    required List<LibraryClassificationCategory> expected,
  }) async {
    resets++;
    lastResetIds = expected.map((category) => category.id).toList();
    await saveGate?.future;
    if (reject) {
      throw const LibraryClassificationFailure(
        "classification_term_changed",
        "Changed",
      );
    }
    terms = [
      for (final term in terms)
        if (lastResetIds.contains(term.category.id))
          LibraryClassificationTerm(
            category: term.defaultCategory!,
            defaultCategory: term.defaultCategory,
            isCustom: term.isCustom,
            isConfigured: term.isConfigured,
          )
        else
          term,
    ];
    return settings;
  }
}
