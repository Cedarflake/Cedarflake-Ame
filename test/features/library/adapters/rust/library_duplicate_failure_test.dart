import "package:cedarflake_ame/features/library/adapters/rust/library_duplicate_failure.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "bridge failures retain Ame code and message across asynchronous and synchronous admission",
    () async {
      const error = rust.ScanError(
        code: "stale_group",
        message: "Group changed",
      );
      final failure = isA<LibraryDuplicateFailure>()
          .having((value) => value.code, "code", error.code)
          .having((value) => value.message, "message", error.message);
      await expectLater(
        invokeDuplicateBridge<void>(() => throw error),
        throwsA(failure),
      );
      await expectLater(
        invokeDuplicateBridge(() => Future<void>.error(error)),
        throwsA(failure),
      );
      await expectLater(
        Stream<void>.error(error).handleError(throwDuplicateFailure),
        emitsError(failure),
      );
    },
  );

  test(
    "unknown failures retain their diagnostic message and successful values pass through",
    () async {
      final error = StateError("Bridge unavailable");
      await expectLater(
        invokeDuplicateBridge<void>(() => throw error),
        throwsA(
          isA<LibraryDuplicateFailure>()
              .having(
                (value) => value.code,
                "code",
                "library_duplicates_failed",
              )
              .having((value) => value.message, "message", error.toString()),
        ),
      );
      final result = Object();
      expect(await invokeDuplicateBridge(() async => result), same(result));
    },
  );
}
