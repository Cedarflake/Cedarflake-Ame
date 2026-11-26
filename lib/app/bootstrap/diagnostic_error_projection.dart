import "dart:io";

import "package:flutter/services.dart";

import "../../features/diagnostics/domain/diagnostic_report.dart";
import "../../features/library/application/ports/library_catalog.dart";
import "../../features/library/domain/library_file_operations.dart";
import "../../features/library/domain/library_classification_review.dart";
import "../../features/library/domain/library_exact_duplicates.dart";
import "../../features/updates/domain/application_update.dart";

DiagnosticEvent diagnosticErrorEvent(
  Object error, {
  DiagnosticComponent component = DiagnosticComponent.application,
}) {
  final code = switch (error) {
    LibraryCatalogFailure(:final code) => code,
    LibraryFileFailure(:final code) => code,
    LibraryClassificationFailure(:final code) => code,
    LibraryDuplicateFailure(:final code) => code,
    PlatformException(:final code) => code,
    ApplicationUpdateFailure(:final code) => code.name,
    _ => error.runtimeType.toString(),
  };
  return DiagnosticEvent(
    occurredAt: DateTime.now().toUtc(),
    component: component,
    phase: "failed",
    code: RegExp(r'^[A-Za-z0-9_.-]{1,80}$').hasMatch(code)
        ? code
        : "unclassified_error",
    nativeCode: _nativeCode(error),
  );
}

DiagnosticNativeCode? _nativeCode(Object error) {
  final osCode = switch (error) {
    FileSystemException(:final osError) => osError?.errorCode,
    OSError(:final errorCode) => errorCode,
    ProcessException(:final errorCode) => errorCode,
    _ => null,
  };
  if (osCode != null) {
    return DiagnosticNativeCode(DiagnosticCodeNamespace.os, "$osCode");
  }
  if (error case DiagnosticFailure(:final nativeCode)) return nativeCode;
  final detail = switch (error) {
    LibraryCatalogFailure(:final message) => message,
    LibraryFileFailure(:final detail) => detail,
    LibraryClassificationFailure(:final message) => message,
    LibraryDuplicateFailure(:final message) => message,
    PlatformException(:final message) => message,
    ApplicationUpdateFailure(:final diagnostic) => diagnostic,
    _ => null,
  };
  if (detail == null) return null;
  final native = RegExp(
    r'\b(OS|WIN32|HRESULT|SQLITE):?\s*(0x[0-9a-fA-F]+|-?[0-9]+)\b',
    caseSensitive: false,
  ).firstMatch(detail.substring(0, detail.length.clamp(0, 4096)));
  if (native == null) return null;
  return DiagnosticNativeCode(
    DiagnosticCodeNamespace.values.byName(native[1]!.toLowerCase()),
    native[2]!,
  );
}
