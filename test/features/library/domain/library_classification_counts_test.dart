import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "classification summaries cannot leak across query or catalog revisions",
    () {
      const counts = LibraryClassificationCounts(
        total: 4,
        pending: 2,
        confirmed: 1,
      );
      final state = LibraryState(
        queryId: "drawing",
        catalogRevision: BigInt.one,
        classificationSummary: LibraryClassificationCountSnapshot(
          "drawing",
          BigInt.one,
          counts,
        ),
      );
      expect(state.classificationCounts, same(counts));
      expect(state.copyWith(queryId: "photo").classificationCounts, isNull);
      expect(
        state.copyWith(catalogRevision: BigInt.two).classificationCounts,
        isNull,
      );
      expect(
        state.copyWith(classificationSummary: null).classificationCounts,
        isNull,
      );
      expect(
        state.copyWith(isLoadingPage: true).classificationCounts,
        same(counts),
      );
    },
  );
}
