import "package:flutter_riverpod/flutter_riverpod.dart";

import "library_classification_lifecycle.dart";

final libraryClassificationConfigurationProvider =
    Provider<LibraryClassificationConfiguration?>((ref) => null);

enum LibraryClassificationMode { disabled, automatic, paused, stopped }

enum LibraryClassificationConfigurationAction { save, enable }

enum LibraryClassificationControl { pause, resume, disable, stop }

class LibraryClassificationCategory {
  const LibraryClassificationCategory({
    required this.id,
    required this.label,
    required this.descriptions,
  });
  final String id;
  final String label;
  final List<String> descriptions;
}

class LibraryClassificationSettings {
  const LibraryClassificationSettings({
    required this.revision,
    required this.dispatchRevision,
    required this.mode,
    required this.categories,
    required this.runId,
    required this.installationSelected,
  });
  final BigInt revision;
  final BigInt dispatchRevision;
  final LibraryClassificationMode mode;
  final List<LibraryClassificationCategory> categories;
  final String? runId;
  final bool installationSelected;
}

abstract interface class LibraryClassificationConfigurationService {
  Future<LibraryClassificationSettings> load();
  Future<List<LibraryClassificationCategory>> initialCategories();
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  });
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  });
}

class LibraryClassificationConfiguration {
  LibraryClassificationConfiguration(this._service, this._lifecycle);
  final LibraryClassificationConfigurationService _service;
  final LibraryClassificationLifecycle _lifecycle;

  Future<LibraryClassificationSettings> load() async {
    try {
      await _lifecycle.waitUntilOpened();
    } on Object {
      // Saved intent remains readable while the lifecycle exposes its startup failure.
    }
    return _service.load();
  }

  Future<void> enableManagedInstallation({
    required String operationId,
    required LibraryClassificationSettings observed,
    required Future<void> Function() commitInstallation,
  }) async {
    final retainedCategories = observed.categories;
    final categories = retainedCategories.isEmpty
        ? await _service.initialCategories()
        : retainedCategories;
    await _lifecycle.changeConfiguration(() async {
      await commitInstallation();
      await _service.save(
        operationId: operationId,
        observed: observed,
        categories: categories,
        action: LibraryClassificationConfigurationAction.enable,
      );
    });
  }

  Future<LibraryClassificationSettings> enable({
    required String operationId,
    required LibraryClassificationSettings observed,
  }) async {
    final categories = observed.categories.isEmpty
        ? await _service.initialCategories()
        : observed.categories;
    await _lifecycle.changeConfiguration(
      () => _service.save(
        operationId: operationId,
        observed: observed,
        categories: categories,
        action: LibraryClassificationConfigurationAction.enable,
      ),
    );
    return _service.load();
  }

  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) async {
    await _lifecycle.changeConfiguration(
      () => _service.changeControl(
        expectedRevision: expectedRevision,
        action: action,
      ),
    );
    return _service.load();
  }
}
