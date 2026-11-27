import "dart:async";
import "dart:io";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_riverpod/misc.dart";

import "../../features/diagnostics/adapters/platform/platform_diagnostic_environment.dart";
import "../../features/diagnostics/adapters/platform/platform_diagnostic_export.dart";
import "../../features/diagnostics/adapters/platform/platform_diagnostic_journal.dart";
import "../../features/diagnostics/adapters/rust/diagnostic_storage_versions.dart";
import "../../features/diagnostics/application/diagnostics_controller.dart";
import "../../features/diagnostics/application/ports/diagnostic_export.dart";
import "../../features/diagnostics/domain/diagnostic_report.dart";
import "../../features/library/application/library_controller.dart";
import "../../features/library/application/library_archive_controller.dart";
import "../../features/library/application/library_archive_state.dart";
import "../../features/library/application/library_cleanup_controller.dart";
import "../../features/library/application/library_cleanup_state.dart";
import "../messages/ame_message.dart";
import "../notifications/ame_notification_controller.dart";
import "../window/ame_build_identity.dart";
import "diagnostic_error_projection.dart";
import "diagnostic_library_snapshot.dart";

class DiagnosticsBindings implements DiagnosticCollector {
  DiagnosticsBindings({Future<Directory> Function()? directory})
    : _journal = PlatformDiagnosticJournal(directory: directory) {
    controller = DiagnosticsController(
      collector: this,
      platform: const PlatformDiagnosticExport(),
    );
    _initializing = _journal.initialize();
    _previousFlutterError = FlutterError.onError;
    _previousPlatformError = PlatformDispatcher.instance.onError;
    FlutterError.onError = _flutterError;
    PlatformDispatcher.instance.onError = _platformError;
    _record(DiagnosticComponent.startup, "starting");
  }

  final PlatformDiagnosticJournal _journal;
  late final Future<void> _initializing;
  late final DiagnosticsController controller;
  final _subscriptions = <ProviderSubscription<Object?>>[];
  ProviderContainer? _container;
  FlutterExceptionHandler? _previousFlutterError;
  bool Function(Object, StackTrace)? _previousPlatformError;
  bool _nativeReady = false;
  bool _closed = false;
  String _startup = "starting";
  List<DiagnosticFact> _versions = [];

  List<Override> get overrides => [
    diagnosticsControllerProvider.overrideWith((ref) => controller),
  ];

  void nativeReady() {
    _nativeReady = true;
    _versions = diagnosticStorageFacts();
  }

  void attach(ProviderContainer container) {
    _container = container;
    _startup = "ready";
    _record(DiagnosticComponent.startup, _startup);
    final archiveSubscription = container.listen(
      libraryArchiveControllerProvider,
      (previous, next) {
        if (next is LibraryArchiveWorking &&
            previous is! LibraryArchiveWorking) {
          controller.dismiss();
        }
      },
    );
    _subscriptions.add(archiveSubscription);
    final cleanupSubscription = container.listen(
      libraryCleanupControllerProvider,
      (previous, next) {
        if (next is LibraryCleanupWorking &&
            previous is! LibraryCleanupWorking) {
          controller.dismiss();
        }
      },
    );
    _subscriptions.add(cleanupSubscription);
    final scanSubscription = container.listen(
      libraryControllerProvider.select(
        (value) => (value.status, value.scanPhase, value.issueCount),
      ),
      (_, next) =>
          _record(DiagnosticComponent.scan, next.$2.name, code: next.$1.name),
    );
    _subscriptions.add(scanSubscription);
    final notificationSubscription = container.listen(
      ameNotificationControllerProvider,
      (previous, next) {
        for (final entry in next.history) {
          if (previous?.history.any(
                (old) =>
                    old.id == entry.id && old.occurredAt == entry.occurredAt,
              ) ??
              false) {
            continue;
          }
          final title = entry.title;
          // Raw messages can contain paths and user-authored labels.
          if (title is AmeLabel) {
            _record(
              DiagnosticComponent.application,
              entry.severity.name,
              code: title.name,
            );
          }
        }
      },
    );
    _subscriptions.add(notificationSubscription);
  }

  void startupFailed(Object error) {
    _container = null;
    _startup = "failed";
    _record(
      DiagnosticComponent.startup,
      _nativeReady
          ? "application_initialization_failed"
          : "native_initialization_failed",
    );
    _journal.record(
      diagnosticErrorEvent(error, component: DiagnosticComponent.startup),
    );
  }

  void _record(DiagnosticComponent component, String phase, {String? code}) {
    if (_closed) return;
    _journal.record(
      DiagnosticEvent(
        occurredAt: DateTime.now().toUtc(),
        component: component,
        phase: phase,
        code: code,
      ),
    );
  }

  void _flutterError(FlutterErrorDetails details) {
    _journal.record(diagnosticErrorEvent(details.exception));
    _previousFlutterError?.call(details);
  }

  bool _platformError(Object error, StackTrace stack) {
    _journal.record(diagnosticErrorEvent(error));
    return _previousPlatformError?.call(error, stack) ?? false;
  }

  @override
  Future<DiagnosticSnapshot> collect() async {
    final environmentReading = const PlatformDiagnosticEnvironment().collect();
    await _initializing.timeout(const Duration(seconds: 1), onTimeout: () {});
    const identity = AmeBuildIdentity.current();
    final facts = [
      DiagnosticFact(
        DiagnosticField.applicationVersion,
        identity.name ?? "development",
      ),
      DiagnosticFact(
        DiagnosticField.applicationBuild,
        identity.number ?? "unavailable",
      ),
      DiagnosticFact(DiagnosticField.startupState, _startup),
      if (_nativeReady) ..._versions,
    ];
    final container = _container;
    final events = <DiagnosticEvent>[];
    if (container != null && !_closed) {
      try {
        final snapshot = await readDiagnosticLibrarySnapshot(container);
        facts.addAll(snapshot.facts);
        events.addAll(snapshot.events);
      } on Object catch (error) {
        events.add(
          diagnosticErrorEvent(error, component: DiagnosticComponent.catalog),
        );
      }
    }
    final environment = await environmentReading;
    return DiagnosticSnapshot(
      capturedAt: DateTime.now().toUtc(),
      facts: [...facts, ...environment.facts],
      graphicsAdapters: environment.graphicsAdapters,
      events: [..._journal.snapshot(), ...events],
      journalAvailable: _journal.available,
    );
  }

  Future<void> close() async {
    if (_closed) return;
    _closed = true;
    for (final subscription in _subscriptions) {
      subscription.close();
    }
    _container = null;
    FlutterError.onError = _previousFlutterError;
    PlatformDispatcher.instance.onError = _previousPlatformError;
    await controller.close();
    controller.dispose();
    await _journal.close();
  }
}
