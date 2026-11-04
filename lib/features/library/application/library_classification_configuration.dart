import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_configuration.dart";
import "library_classification_lifecycle.dart";
import "ports/library_classification_configuration.dart";

final libraryClassificationConfigurationProvider =
    Provider<LibraryClassificationConfiguration?>(
      (ref) => throw StateError(
        "Library classification configuration binding is required",
      ),
    );

class LibraryClassificationConfiguration {
  LibraryClassificationConfiguration(this._service, this._lifecycle);
  final LibraryClassificationConfigurationService _service;
  final LibraryClassificationLifecycle _lifecycle;

  Future<LibraryClassificationSettings> load() => _load(_service.load);

  Future<LibraryClassificationSettings> loadNavigation() =>
      _load(_service.loadNavigation);

  Future<LibraryClassificationSettings> _load(
    Future<LibraryClassificationSettings> Function() read,
  ) async {
    try {
      await _lifecycle.waitUntilOpened();
    } on Object {
      // Saved intent remains readable while the lifecycle exposes its startup failure.
    }
    return read();
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
