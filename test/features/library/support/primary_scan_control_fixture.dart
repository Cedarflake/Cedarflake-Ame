import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scan_control.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "retained_scan_fixture.dart";

class PrimaryScanControlFixture {
  PrimaryScanControlFixture() {
    final catalog = RetainedScanCatalog(scanner);
    container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            catalog.snapshot(const LibraryGalleryQuery()),
          ),
        ),
      ],
    );
    addTearDown(scanner.dispose);
    addTearDown(container.dispose);
  }
  final scanner = PrimaryScanRegistrationScanner()..checkpoint = null;
  late final ProviderContainer container;
  LibraryController get controller =>
      container.read(libraryControllerProvider.notifier);
  LibraryState get state => container.read(libraryControllerProvider);

  Future<String> start() async {
    controller;
    await flushRetainedScanMicrotasks();
    await controller.scanDirectory(retainedScanRoot.path);
    return state.scanId!;
  }

  Future<void> request(LibraryScanCommand command) async {
    if (command == LibraryScanCommand.cancel) {
      await controller.cancelScan();
    } else {
      controller.pauseScan();
    }
  }
}

class PrimaryScanRegistrationScanner extends RetainedScanScanner {
  final Set<String> registered = {};
  final List<LibraryScanCommand> commands = [];
  final List<String> controlIds = [];
  final Map<String, StreamController<LibraryScanUpdate>> _events = {};
  bool failNextControl = false;

  bool _control(String id, LibraryScanCommand command) {
    controlIds.add(id);
    commands.add(command);
    if (failNextControl) {
      failNextControl = false;
      throw StateError("controlled bridge failure");
    }
    return registered.contains(id);
  }

  @override
  bool cancel(String scanId) => _control(scanId, LibraryScanCommand.cancel);
  @override
  bool pause(String scanId) => _control(scanId, LibraryScanCommand.pause);
  @override
  bool suspend(String scanId) => _control(scanId, LibraryScanCommand.suspend);
  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return _events
        .putIfAbsent(
          scanId,
          () => StreamController<LibraryScanUpdate>(sync: true),
        )
        .stream;
  }

  void started(String id) {
    registered.add(id);
    emit(id, LibraryScanStarted(scanId: id, rootPath: retainedScanRoot.path));
  }

  void emit(String id, LibraryScanUpdate update) => _events[id]!.add(update);
  void fail(String id) =>
      _events[id]!.addError(StateError("controlled stream failure"));
  Future<void> close(String id) {
    final stream = _events[id]!;
    // Capture the test-zone completion before synchronous onDone cancels the stream.
    final done = stream.done;
    unawaited(stream.close());
    return done;
  }

  @override
  void dispose() {
    for (final stream in _events.values) {
      if (!stream.isClosed) {
        unawaited(stream.close());
      }
    }
    super.dispose();
  }
}
