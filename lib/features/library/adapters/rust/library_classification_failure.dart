import "../../../../src/rust/domain.dart" as rust;
import "../../domain/library_classification_review.dart";

Future<T> invokeClassificationBridge<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on Object catch (error, stack) {
    _throwFailure(error, stack);
  }
}

T invokeClassificationSync<T>(T Function() operation) {
  try {
    return operation();
  } on Object catch (error, stack) {
    _throwFailure(error, stack);
  }
}

Never _throwFailure(Object error, StackTrace stack) {
  final failure = switch (error) {
    LibraryClassificationFailure() => error,
    rust.ScanError(:final code, :final message) => LibraryClassificationFailure(
      code,
      message,
    ),
    _ => LibraryClassificationFailure(
      "classification_failed",
      error.toString(),
    ),
  };
  Error.throwWithStackTrace(failure, stack);
}
