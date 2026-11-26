import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/bootstrap/diagnostic_error_projection.dart";
import "package:cedarflake_ame/features/diagnostics/adapters/platform/diagnostic_encoding.dart";
import "package:cedarflake_ame/features/library/application/ports/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_file_operations.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "filesystem errors retain OS code without private messages or paths",
    () {
      const error = FileSystemException(
        "Private filename.png",
        r"C:\Users\PrivateName\Pictures\filename.png",
        OSError("Private native message", 32),
      );
      final event = diagnosticErrorEvent(error);
      expect(event.code, "FileSystemException");
      expect(event.nativeCode?.label, "OS:32");
      final serialized = jsonEncode(encodeDiagnosticEvent(event));
      expect(serialized, isNot(contains("Private")));
      expect(serialized, isNot(contains("filename.png")));
    },
  );

  test("domain error projection keeps codes but discards descriptions", () {
    final cases = <(Object, String, String)>[
      (
        const LibraryCatalogFailure(
          code: "catalog_open",
          message: "Private database HRESULT:0x80070005",
        ),
        "catalog_open",
        "HRESULT:0x80070005",
      ),
      (
        const LibraryFileFailure("file_locked", "Private filename OS:32"),
        "file_locked",
        "OS:32",
      ),
      (
        const LibraryClassificationFailure(
          "runtime_unavailable",
          "Private model WIN32:126",
        ),
        "runtime_unavailable",
        "WIN32:126",
      ),
    ];
    cases.add((
      const LibraryDuplicateFailure(
        code: "similarity_read",
        message: "Private path SQLITE:14",
      ),
      "similarity_read",
      "SQLITE:14",
    ));
    for (final (error, code, native) in cases) {
      final event = diagnosticErrorEvent(error);
      expect(event.code, code);
      expect(event.nativeCode?.label, native);
      expect(
        jsonEncode(encodeDiagnosticEvent(event)),
        isNot(contains("Private")),
      );
    }
  });

  test("platform details and invalid error identifiers are excluded", () {
    final event = diagnosticErrorEvent(
      PlatformException(
        code: r"C:\Users\PrivateName\photo.png",
        message: "Private account OS:5",
        details: {"token": "private-token"},
      ),
    );
    expect(event.code, "unclassified_error");
    expect(event.nativeCode?.label, "OS:5");
    expect(
      jsonEncode(encodeDiagnosticEvent(event)),
      isNot(contains("Private")),
    );
    expect(
      jsonEncode(encodeDiagnosticEvent(event)),
      isNot(contains("private-token")),
    );
  });

  test("unknown exceptions are never stringified", () {
    final event = diagnosticErrorEvent(_PrivateError());
    expect(event.code, "_PrivateError");
    expect(event.nativeCode, isNull);
  });
}

class _PrivateError {
  @override
  String toString() => throw StateError("Private error description");
}
