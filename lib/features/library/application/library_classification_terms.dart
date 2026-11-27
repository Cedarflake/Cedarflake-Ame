import "dart:math";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_configuration.dart";
import "../domain/library_classification_term.dart";
import "library_classification_lifecycle.dart";
import "library_foreground_reads.dart";
import "ports/library_classification_terms.dart";

final libraryClassificationTermsServiceProvider =
    Provider<LibraryClassificationTermsService?>(
      (ref) =>
          throw StateError("Library classification terms binding is required"),
    );

final libraryClassificationTermsRevisionProvider =
    NotifierProvider<LibraryClassificationTermsRevision, int>(
      LibraryClassificationTermsRevision.new,
    );

class LibraryClassificationTermsRevision extends Notifier<int> {
  @override
  int build() => 0;
  void changed() => state++;
}

final libraryClassificationTermsProvider =
    Provider<LibraryClassificationTerms?>((ref) {
      final service = ref.watch(libraryClassificationTermsServiceProvider);
      final lifecycle = ref.read(libraryClassificationLifecycleProvider);
      if (service == null || lifecycle == null) return null;
      return LibraryClassificationTerms(
        service,
        changeConfiguration: lifecycle.changeConfiguration,
        onChanged: () => ref
            .read(libraryClassificationTermsRevisionProvider.notifier)
            .changed(),
      );
    });

String newClassificationTermOperationId() {
  final random = Random.secure();
  return List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
  ).join();
}

class LibraryClassificationTerms {
  LibraryClassificationTerms(
    this._service, {
    required this._changeConfiguration,
    required this._onChanged,
  });

  final LibraryClassificationTermsService _service;
  final Future<LibraryClassificationSettings> Function(
    Future<LibraryClassificationSettings> Function(),
  )
  _changeConfiguration;
  final VoidCallback _onChanged;

  Future<List<LibraryClassificationTerm>> load() =>
      libraryForegroundReads.run(_service.load);

  Future<LibraryClassificationTermChange> save({
    required String operationId,
    required LibraryClassificationTerm? original,
    required LibraryClassificationCategory replacement,
  }) async {
    final settings = await _change(
      () => _service.save(
        operationId: operationId,
        expected: original != null && original.isConfigured
            ? original.category
            : null,
        replacement: replacement,
      ),
      semantic:
          original == null ||
          !original.isConfigured ||
          !_sameDescriptions(original.category, replacement),
    );
    return _result(settings, replacement.id, original);
  }

  Future<LibraryClassificationTermChange> reset({
    required String operationId,
    required LibraryClassificationTerm original,
  }) async {
    final defaults = original.defaultCategory;
    if (defaults == null) throw StateError("Term has no bundled default");
    if (!original.isConfigured) {
      throw StateError("Unconfigured terms can only reset their draft");
    }
    final settings = await _change(
      () =>
          _service.reset(operationId: operationId, expected: original.category),
      semantic: !_sameDescriptions(original.category, defaults),
    );
    return _result(settings, original.category.id, original);
  }

  Future<LibraryClassificationTermChange> remove({
    required String operationId,
    required LibraryClassificationTerm original,
  }) async {
    await _change(
      () => _service.remove(
        operationId: operationId,
        expected: original.category,
      ),
      semantic: true,
    );
    return LibraryClassificationTermChange(original.category.id, null);
  }

  Future<LibraryClassificationSettings> _change(
    Future<LibraryClassificationSettings> Function() action, {
    required bool semantic,
  }) async {
    final settings = await (semantic ? _changeConfiguration(action) : action());
    _onChanged();
    return settings;
  }

  LibraryClassificationTermChange _result(
    LibraryClassificationSettings settings,
    String id,
    LibraryClassificationTerm? original,
  ) => LibraryClassificationTermChange(
    id,
    LibraryClassificationTerm(
      category: settings.categories.singleWhere(
        (category) => category.id == id,
      ),
      defaultCategory: original?.defaultCategory,
      isCustom: original?.isCustom ?? true,
    ),
  );

  bool _sameDescriptions(
    LibraryClassificationCategory a,
    LibraryClassificationCategory b,
  ) => setEquals(a.descriptions.toSet(), b.descriptions.toSet());
}
