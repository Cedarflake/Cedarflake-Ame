import "../../../../src/rust/domain.dart" as rust;
import "../../domain/library_exact_duplicates.dart";

T invokeDuplicateSync<T>(T Function() operation) {
  try {
    return operation();
  } on Object catch (error, stack) {
    throwDuplicateFailure(error, stack);
  }
}

Future<T> invokeDuplicateBridge<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on Object catch (error, stack) {
    throwDuplicateFailure(error, stack);
  }
}

Never throwDuplicateFailure(Object error, StackTrace stack) {
  final failure = switch (error) {
    LibraryDuplicateFailure() => error,
    rust.ScanError() => LibraryDuplicateFailure(
      code: error.code,
      message: error.message,
    ),
    _ => LibraryDuplicateFailure(
      code: "library_duplicates_failed",
      message: error.toString(),
    ),
  };
  Error.throwWithStackTrace(failure, stack);
}
