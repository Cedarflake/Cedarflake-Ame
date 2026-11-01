import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "library_classification.dart";
import "library_classification_environment.dart";
import "library_classification_session_lifetime.dart";
import "library_foreground_reads.dart";

final libraryClassificationLifecycleProvider =
    Provider<LibraryClassificationLifecycle?>((ref) => null);

class LibraryClassificationLifecycle extends ChangeNotifier {
  LibraryClassificationLifecycle(
    LibraryClassification classification, {
    required LibraryForegroundReads foregroundReads,
  }) {
    _environment = LibraryClassificationEnvironment(
      foregroundReads,
      _reportError,
    );
    _sessions = LibraryClassificationSessionLifetime(
      classification,
      _environment,
      _reportError,
    );
  }

  late final LibraryClassificationEnvironment _environment;
  late final LibraryClassificationSessionLifetime _sessions;
  Object? _error;

  Object? get error => _error;

  void reportEnvironmentError(Object error, StackTrace stackTrace) =>
      _reportError(error, stackTrace);

  Future<void> waitUntilOpened() => _sessions.waitUntilOpened();
  void startInBackground() => _sessions.startInBackground();
  Future<T> changeConfiguration<T>(Future<T> Function() change) =>
      _sessions.changeConfiguration(change);

  void updateCatalog({
    required BigInt revision,
    required Map<String, BigInt> availableRoots,
    required bool isBusy,
  }) => _environment.updateCatalog(
    revision: revision,
    availableRoots: availableRoots,
    isBusy: isBusy,
  );

  void setSuspended(bool suspended) => _environment.setSuspended(suspended);
  void memoryPressure() => _environment.memoryPressure();
  void browsingActivity() => _environment.browsingActivity();
  Future<void> close() => _sessions.close();

  void _reportError(Object error, StackTrace stackTrace) {
    _error = error;
    debugPrint("[Ame classification] lifecycle failed: ${error.runtimeType}");
    notifyListeners();
  }
}
