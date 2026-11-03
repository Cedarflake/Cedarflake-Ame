import "../../../../src/rust/domain.dart" as rust;
import "../../domain/library_organization.dart";

Future<T> invokeOrganizationBridge<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on rust.ScanError catch (error, stack) {
    Error.throwWithStackTrace(
      LibraryOrganizationFailure(error.code, error.message),
      stack,
    );
  }
}
