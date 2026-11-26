import "dart:io";
import "dart:isolate";

import "package:file_selector/file_selector.dart";

import "../../application/ports/diagnostic_export.dart";
import "../../domain/diagnostic_report.dart";
import "diagnostic_encoding.dart";

class PlatformDiagnosticExport implements DiagnosticExportPlatform {
  const PlatformDiagnosticExport();

  @override
  Future<String?> chooseDestination(DiagnosticSaveLabels labels) async {
    try {
      final timestamp = DateTime.now()
          .toIso8601String()
          .substring(0, 19)
          .replaceAll(":", "-");
      return (await getSaveLocation(
        acceptedTypeGroups: [
          XTypeGroup(label: labels.type, extensions: ["zip"]),
        ],
        suggestedName: "Ame-diagnostics-$timestamp.zip",
        confirmButtonText: labels.save,
      ))?.path;
    } on Object catch (error) {
      throw _failure(DiagnosticFailureKind.destination, error);
    }
  }

  @override
  Future<void> write(
    String destination,
    DiagnosticSnapshot snapshot,
    DiagnosticCancellation cancellation,
  ) async {
    Directory? temporary;
    File? pending;
    try {
      cancellation.check();
      final target = File(destination);
      if (!target.isAbsolute || !destination.toLowerCase().endsWith(".zip")) {
        throw const DiagnosticFailure(DiagnosticFailureKind.destination);
      }
      final bytes = await Isolate.run(() => encodeDiagnosticArchive(snapshot));
      cancellation.check();
      temporary = await target.parent.createTemp(".ame-diagnostics-");
      pending = File("${temporary.path}${Platform.pathSeparator}report.zip");
      await pending.writeAsBytes(bytes, flush: true);
      cancellation.beginCommit();
      await pending.rename(destination);
    } on DiagnosticCancelled {
      rethrow;
    } on DiagnosticFailure {
      rethrow;
    } on Object catch (error) {
      throw _failure(DiagnosticFailureKind.writing, error);
    } finally {
      try {
        if (temporary != null && await temporary.exists()) {
          if (pending != null && await pending.exists()) await pending.delete();
          await temporary.delete();
        }
      } on FileSystemException {
        // Cleanup cannot change the result of the destination commit.
      }
    }
  }

  @override
  Future<void> openFolder(String destination) async {
    try {
      final parent = File(destination).parent;
      if (!await parent.exists()) {
        throw const FileSystemException("Destination unavailable");
      }
      await Process.start("explorer.exe", [
        parent.path,
      ], mode: ProcessStartMode.detached);
    } on Object catch (error) {
      throw _failure(DiagnosticFailureKind.openingFolder, error);
    }
  }
}

DiagnosticFailure _failure(DiagnosticFailureKind kind, Object error) {
  final native = switch (error) {
    FileSystemException(:final osError) => osError?.errorCode,
    ProcessException(:final errorCode) => errorCode,
    _ => null,
  };
  return DiagnosticFailure(
    kind,
    nativeCode: native == null
        ? null
        : DiagnosticNativeCode(DiagnosticCodeNamespace.os, "$native"),
  );
}
