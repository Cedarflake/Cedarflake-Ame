import "package:cedarflake_ame/features/library/adapters/rust/library_classification_failure.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "native classification failures retain code, message and stack",
    () async {
      const failure = rust.ScanError(
        code: "stale_configuration",
        message: "Changed revision",
      );
      final stack = StackTrace.fromString("native-classification-boundary");
      try {
        await invokeClassificationBridge<void>(
          () => Future.error(failure, stack),
        );
        fail("Expected an Ame classification failure");
      } on LibraryClassificationFailure catch (error, caughtStack) {
        expect(error.code, failure.code);
        expect(error.message, failure.message);
        expect(caughtStack.toString(), stack.toString());
      }
      expect(
        () => invokeClassificationSync<void>(() => throw failure),
        throwsA(
          isA<LibraryClassificationFailure>().having(
            (e) => e.code,
            "code",
            failure.code,
          ),
        ),
      );
    },
  );

  test(
    "Ame failure identity survives and foreign failures stay in the adapter",
    () async {
      const failure = LibraryClassificationFailure("cancelled", "Cancelled");
      await expectLater(
        invokeClassificationBridge<void>(() async => throw failure),
        throwsA(same(failure)),
      );
      expect(
        () => invokeClassificationSync<void>(() => throw failure),
        throwsA(same(failure)),
      );
      expect(
        () => invokeClassificationSync<void>(
          () => throw StateError("closed native handle"),
        ),
        throwsA(
          isA<LibraryClassificationFailure>().having(
            (e) => e.code,
            "code",
            "classification_failed",
          ),
        ),
      );
    },
  );
}
