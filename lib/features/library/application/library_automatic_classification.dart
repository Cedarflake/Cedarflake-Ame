import "dart:math";

import "../domain/library_classification_configuration.dart";
import "ports/library_classification.dart";
import "ports/library_classification_configuration.dart";

class LibraryAutomaticClassification implements LibraryClassification {
  const LibraryAutomaticClassification(
    this._classification,
    this._configuration,
  );

  final LibraryClassification _classification;
  final LibraryClassificationConfigurationService _configuration;

  @override
  Future<LibraryClassificationSession?> open() async {
    final observed = await _configuration.load();
    if (!observed.installationSelected) return null;
    switch (observed.mode) {
      case LibraryClassificationMode.automatic:
        break;
      case LibraryClassificationMode.paused:
        await _configuration.changeControl(
          expectedRevision: observed.dispatchRevision,
          action: LibraryClassificationControl.resume,
        );
      case LibraryClassificationMode.disabled:
      case LibraryClassificationMode.stopped:
        final categories = observed.categories.isEmpty
            ? await _configuration.initialCategories()
            : observed.categories;
        final random = Random.secure();
        final operationId = List.generate(
          16,
          (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
        ).join();
        await _configuration.save(
          operationId: operationId,
          observed: observed,
          categories: categories,
          action: LibraryClassificationConfigurationAction.enable,
        );
    }
    return _classification.open();
  }
}
