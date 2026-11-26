import "dart:async";
import "dart:convert";
import "dart:io";

import "package:archive/archive.dart";
import "package:cedarflake_ame/features/diagnostics/adapters/platform/platform_diagnostic_export.dart";
import "package:cedarflake_ame/features/diagnostics/adapters/platform/platform_diagnostic_journal.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("startup events wait for retained journal before persistence", () async {
    final root = await Directory.systemTemp.createTemp(
      "ame-diagnostics-startup-",
    );
    try {
      final previous = PlatformDiagnosticJournal(directory: () async => root);
      await previous.initialize();
      previous.record(_snapshot().events.single);
      await previous.close();
      final directory = Completer<Directory>();
      final current = PlatformDiagnosticJournal(
        directory: () => directory.future,
      );
      final initializing = current.initialize();
      expect(current.available, isFalse);
      current.record(
        DiagnosticEvent(
          occurredAt: DateTime.utc(2026, 11, 27),
          component: DiagnosticComponent.startup,
          phase: "starting",
        ),
      );
      await Future<void>.delayed(const Duration(milliseconds: 300));
      directory.complete(root);
      await initializing;
      await Future<void>.delayed(const Duration(milliseconds: 300));
      final file = File("${root.path}${Platform.pathSeparator}events.json");
      final saved = jsonDecode(await file.readAsString()) as List<Object?>;
      expect(saved.length, 2);
      expect(current.snapshot().first.code, "catalog_database_error");
      expect(current.snapshot().last.phase, "starting");
      await current.close();
    } finally {
      await root.delete(recursive: true);
    }
  });

  test(
    "ZIP includes useful failure evidence without source or identity fields",
    () async {
      final root = await Directory.systemTemp.createTemp(
        "ame-diagnostics-test-",
      );
      try {
        final target = File("${root.path}${Platform.pathSeparator}诊断.zip");
        await const PlatformDiagnosticExport().write(
          target.path,
          _snapshot(),
          DiagnosticCancellation(),
        );
        final archive = ZipDecoder().decodeBytes(
          await target.readAsBytes(),
          verify: true,
        );
        expect(archive.files.map((file) => file.name), [
          "summary.txt",
          "diagnostics.json",
        ]);
        final summary = utf8.decode(archive.findFile("summary.txt")!.content);
        expect(summary, contains("OS:32"));
        expect(summary, contains("catalog_database_error"));
        final report =
            jsonDecode(
                  utf8.decode(archive.findFile("diagnostics.json")!.content),
                )
                as Map<String, dynamic>;
        expect(report["facts"], {"applicationVersion": "0.1.0-beta.1"});
        expect(
          report.keys,
          unorderedEquals([
            "format",
            "capturedAt",
            "facts",
            "journalAvailable",
            "events",
          ]),
        );
        expect(await root.list().length, 1);
      } finally {
        await root.delete(recursive: true);
      }
    },
  );

  test("cancelled export leaves a prior destination byte-identical", () async {
    final root = await Directory.systemTemp.createTemp("ame-diagnostics-test-");
    try {
      final target = File("${root.path}${Platform.pathSeparator}report.zip");
      await target.writeAsBytes([7, 8, 9]);
      final cancellation = DiagnosticCancellation()..cancel();
      await expectLater(
        const PlatformDiagnosticExport().write(
          target.path,
          _snapshot(),
          cancellation,
        ),
        throwsA(isA<DiagnosticCancelled>()),
      );
      expect(await target.readAsBytes(), [7, 8, 9]);
      expect(await root.list().length, 1);
    } finally {
      await root.delete(recursive: true);
    }
  });

  test(
    "diagnostic values reject paths, arbitrary messages and oversized tokens",
    () {
      for (final value in [
        r"C:\Users\Alice\secret.jpg",
        "Bearer abc@token",
        "failure\nprivate text",
        "x" * 161,
      ]) {
        expect(
          () => DiagnosticFact(DiagnosticField.classificationRuntime, value),
          throwsArgumentError,
        );
      }
      expect(
        () => DiagnosticEvent(
          occurredAt: DateTime.utc(2026),
          component: DiagnosticComponent.catalog,
          phase: "failed",
          code: "C:/private.jpg",
        ),
        throwsArgumentError,
      );
    },
  );

  test("journal retains the bounded newest history across restart", () async {
    final root = await Directory.systemTemp.createTemp(
      "ame-diagnostics-journal-",
    );
    try {
      final journal = PlatformDiagnosticJournal(directory: () async => root);
      await journal.initialize();
      for (var index = 0; index < 300; index++) {
        journal.record(
          DiagnosticEvent(
            occurredAt: DateTime.utc(2026, 11, 26),
            component: DiagnosticComponent.scan,
            phase: "phase$index",
          ),
        );
      }
      await journal.close();
      final reopened = PlatformDiagnosticJournal(directory: () async => root);
      await reopened.initialize();
      expect(reopened.available, isTrue);
      expect(reopened.snapshot().length, 256);
      expect(reopened.snapshot().first.phase, "phase44");
      expect(reopened.snapshot().last.phase, "phase299");
      await reopened.close();
      final files = await root.list().toList();
      expect(files.length, 1);
      expect(
        await File(files.single.path).length(),
        lessThanOrEqualTo(128 * 1024),
      );
    } finally {
      await root.delete(recursive: true);
    }
  });

  test(
    "unavailable journal preserves current-session evidence in memory",
    () async {
      final journal = PlatformDiagnosticJournal(
        directory: () async => throw const FileSystemException("Unavailable"),
      );
      await journal.initialize();
      journal.record(_snapshot().events.single);
      expect(journal.available, isFalse);
      expect(journal.snapshot().single.code, "catalog_database_error");
      await journal.close();
    },
  );
}

DiagnosticSnapshot _snapshot() => DiagnosticSnapshot(
  capturedAt: DateTime.utc(2026, 11, 26),
  facts: [DiagnosticFact(DiagnosticField.applicationVersion, "0.1.0-beta.1")],
  events: [
    DiagnosticEvent(
      occurredAt: DateTime.utc(2026, 11, 26),
      component: DiagnosticComponent.catalog,
      phase: "failed",
      code: "catalog_database_error",
      nativeCode: DiagnosticNativeCode(DiagnosticCodeNamespace.os, "32"),
    ),
  ],
);
