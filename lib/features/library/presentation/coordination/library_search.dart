import "dart:async";

import "package:material_ui/material_ui.dart";

import "../../application/library_query_update.dart";
import "../../application/library_semantic_search.dart";
import "../../application/ports/library_semantic_search.dart";
import "../../domain/library_models.dart";
import "../../domain/library_semantic_search.dart";
import "../../domain/library_view_preferences.dart";

class LibrarySearch extends ChangeNotifier {
  LibrarySearch({
    required this.text,
    required this.readQuery,
    required this.publish,
    required this.onFailure,
    required this.supersedePublication,
    required this.allowContent,
    required this.onModeChanged,
    required LibrarySearchMode initialMode,
    LibrarySemanticSearch? service,
  }) : _lastEditingValue = text.value,
       _session = service == null
           ? null
           : LibrarySemanticSearchSession(service),
       _mode = service == null || text.text.isNotEmpty
           ? LibrarySearchMode.fileName
           : initialMode {
    text.addListener(_edited);
  }

  final TextEditingController text;
  final LibraryGalleryQuery Function() readQuery;
  final Future<LibraryQueryUpdateOutcome> Function(
    LibraryGalleryQuery query, {
    required bool preservePosition,
  })
  publish;
  final void Function(Object? error) onFailure;
  final VoidCallback supersedePublication;
  final bool Function() allowContent;
  final ValueChanged<LibrarySearchMode> onModeChanged;
  final LibrarySemanticSearchSession? _session;
  TextEditingValue _lastEditingValue;
  LibrarySearchMode _mode;
  LibrarySearchMode get selectedMode => _mode;
  LibrarySearchMode get mode => available ? _mode : LibrarySearchMode.fileName;
  bool wholeLibrary = false;
  bool loading = false;
  bool hasUpdates = false;
  Object? error;
  LibrarySemanticInput? _input;
  LibraryGalleryQuery? _scope;
  Timer? _debounce;
  Timer? _poll;
  bool _polling = false;
  bool _disposed = false;
  int _generation = 0;

  bool get available => _session != null && allowContent();
  bool get active => _input != null || text.text.trim().isNotEmpty;
  bool get related => _input is LibrarySemanticReference;
  String? get referenceName => switch (_input) {
    LibrarySemanticReference(:final name) => name,
    _ => null,
  };

  void _edited() {
    final value = text.value;
    final previous = _lastEditingValue;
    _lastEditingValue = value;
    if (value.text == previous.text && value.composing == previous.composing) {
      return;
    }
    _debounce?.cancel();
    cancelPending();
    error = null;
    onFailure(null);
    notifyListeners();
    if (_disposed ||
        (text.value.composing.isValid && !text.value.composing.isCollapsed)) {
      return;
    }
    _input = null;
    notifyListeners();
    _debounce = Timer(
      const Duration(milliseconds: 250),
      () => unawaited(submit()),
    );
  }

  Future<void> submit() async {
    _debounce?.cancel();
    if (_disposed ||
        (text.value.composing.isValid && !text.value.composing.isCollapsed)) {
      return;
    }
    if (_input is! LibrarySemanticReference && text.text.trim().isEmpty) {
      await clear();
      return;
    }
    _scope ??= readQuery().copyWith(searchText: "", semanticResults: null);
    await _search(_effectiveScope());
  }

  LibraryGalleryQuery _effectiveScope() {
    final scope =
        _scope ?? readQuery().copyWith(searchText: "", semanticResults: null);
    return wholeLibrary
        ? scope.copyWith(
            collection: const LibraryGalleryCollection.library(),
            rootId: null,
            folderRelativePath: null,
            organizationQueue: null,
            newSinceAdmission: null,
          )
        : scope;
  }

  Future<LibraryQueryUpdateOutcome> _search(
    LibraryGalleryQuery scope, {
    bool preservePosition = false,
  }) async {
    cancelPending();
    final generation = _generation;
    error = null;
    loading = true;
    hasUpdates = false;
    notifyListeners();
    try {
      LibraryGalleryQuery query;
      if (mode == LibrarySearchMode.fileName &&
          _input is! LibrarySemanticReference) {
        query = scope.copyWith(searchText: text.text, semanticResults: null);
      } else {
        final input = _input ?? LibrarySemanticDescription(text.text);
        final result = await _session!.search(scope, input);
        if (_disposed || generation != _generation || result == null) {
          return LibraryQueryUpdateOutcome.superseded;
        }
        _input = input;
        query = scope.copyWith(searchText: "", semanticResults: result);
      }
      if (_disposed || generation != _generation) {
        return LibraryQueryUpdateOutcome.superseded;
      }
      final retainsLiteralSearch =
          query.semanticResults == null && readQuery().semanticResults == null;
      final outcome = await publish(
        query,
        preservePosition: preservePosition || retainsLiteralSearch,
      );
      if (generation == _generation &&
          outcome == LibraryQueryUpdateOutcome.applied) {
        onFailure(null);
        _poll?.cancel();
        if (query.semanticResults != null) {
          _poll = Timer.periodic(
            const Duration(seconds: 3),
            (_) => unawaited(_pollIndex()),
          );
        }
      }
      return outcome;
    } on Object catch (failure) {
      if (!_disposed && generation == _generation) {
        error = failure;
        onFailure(failure);
      }
      return LibraryQueryUpdateOutcome.failed;
    } finally {
      if (!_disposed && generation == _generation) {
        loading = false;
        notifyListeners();
      }
    }
  }

  Future<void> _pollIndex() async {
    if (_polling || _disposed || _session == null) return;
    _polling = true;
    final generation = _generation;
    try {
      final status = await _session.service.indexStatus();
      final results = readQuery().semanticResults;
      if (!_disposed &&
          generation == _generation &&
          results != null &&
          status != null) {
        final catalogRevision = status.catalogRevision;
        final changed =
            status.featureGeneration > results.featureGeneration ||
            (catalogRevision != null &&
                catalogRevision > results.catalogRevision);
        if (status.blocked && error == null) {
          error = LibrarySemanticIndexFailure(status.issueCode);
          onFailure(error);
          notifyListeners();
        }
        if (changed != hasUpdates) {
          hasUpdates = changed;
          notifyListeners();
        }
      }
    } on Object catch (failure) {
      if (!_disposed && generation == _generation && error == null) {
        error = failure;
        onFailure(failure);
        notifyListeners();
      }
    } finally {
      _polling = false;
    }
  }

  Future<LibraryQueryUpdateOutcome> changeScope(
    LibraryGalleryQuery query,
  ) async {
    _scope = query.copyWith(searchText: "", semanticResults: null);
    wholeLibrary = false;
    return active
        ? _search(_scope!)
        : publish(_scope!, preservePosition: false);
  }

  Future<LibraryQueryUpdateOutcome> changeFilters(LibraryGalleryQuery query) {
    _scope = (_scope ?? query).copyWith(
      metadataFilter: query.metadataFilter,
      includeDescendants: query.includeDescendants,
      duplicateMode: query.duplicateMode,
      groupFilter: query.groupFilter,
      searchText: "",
      semanticResults: null,
    );
    return active
        ? _search(_effectiveScope(), preservePosition: true)
        : publish(_effectiveScope(), preservePosition: true);
  }

  void adopt(LibraryGalleryQuery query) {
    cancelPending();
    _poll?.cancel();
    _scope = null;
    _input = null;
    wholeLibrary = false;
    error = null;
    hasUpdates = false;
    _replaceText(query.searchText);
    if (query.searchText.isNotEmpty) _mode = LibrarySearchMode.fileName;
    notifyListeners();
  }

  void selectMode(LibrarySearchMode value) {
    if (value == LibrarySearchMode.content && !available) return;
    _mode = value;
    onModeChanged(value);
    _input = null;
    notifyListeners();
    unawaited(submit());
  }

  void selectScope(bool entire) {
    wholeLibrary = entire;
    notifyListeners();
    if (active) unawaited(submit());
  }

  void findRelated(LibraryAsset asset) {
    if (!available) return;
    _replaceText("");
    _input = LibrarySemanticReference((
      assetId: asset.assetId,
      locationId: asset.locationId,
      sourceGeneration: asset.sourceGeneration,
    ), asset.relativePath.replaceAll("\\", "/").split("/").last);
    _scope ??= readQuery().copyWith(searchText: "", semanticResults: null);
    unawaited(submit());
  }

  Future<void> dismissFailure(Object failure) async {
    if (_disposed ||
        loading ||
        !active ||
        !identical(error, failure) ||
        failure is LibrarySemanticIndexFailure) {
      return;
    }
    await clear();
  }

  Future<void> clear() async {
    cancelPending();
    final generation = _generation;
    _debounce?.cancel();
    _poll?.cancel();
    _input = null;
    error = null;
    hasUpdates = false;
    onFailure(null);
    final scope = _effectiveScope();
    _scope = null;
    _replaceText("");
    try {
      await publish(
        scope.copyWith(searchText: "", semanticResults: null),
        preservePosition: readQuery().semanticResults == null,
      );
    } on Object catch (failure) {
      if (!_disposed && generation == _generation) {
        error = failure;
        onFailure(failure);
      }
    } finally {
      if (!_disposed && generation == _generation) notifyListeners();
    }
  }

  void cancelPending() {
    _generation += 1;
    _session?.cancel();
    supersedePublication();
    loading = false;
  }

  Future<void> retry() async {
    if (_disposed || loading) return;
    final recoverIndex =
        error != null && (mode == LibrarySearchMode.content || related);
    cancelPending();
    final generation = _generation;
    error = null;
    loading = true;
    hasUpdates = false;
    onFailure(null);
    notifyListeners();
    try {
      if (recoverIndex) {
        await _session?.service.retryIndex();
      }
      if (_disposed || generation != _generation) return;
      await _search(_effectiveScope(), preservePosition: true);
    } on Object catch (failure) {
      if (!_disposed && generation == _generation) {
        error = failure;
        onFailure(failure);
        notifyListeners();
      }
    } finally {
      if (!_disposed && generation == _generation) {
        loading = false;
        notifyListeners();
      }
    }
  }

  void _replaceText(String value) {
    text.removeListener(_edited);
    text.text = value;
    _lastEditingValue = text.value;
    text.addListener(_edited);
  }

  @override
  void dispose() {
    _disposed = true;
    text.removeListener(_edited);
    _debounce?.cancel();
    _poll?.cancel();
    _session?.dispose();
    super.dispose();
  }
}
