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

class ClassificationTermsFixture implements LibraryClassificationTermsService {
  List<LibraryClassificationTerm> terms = [starterTerm];
  int retired = 0;
  int changes = 0;
  int loads = 0;
  int saves = 0;
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
    categories: terms.map((term) => term.category).toList(),
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
}
