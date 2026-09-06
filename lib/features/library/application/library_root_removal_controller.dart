import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_catalog_publication.dart";
import "library_viewport_controller.dart";

class PreparedLibraryRootRemoval {
  PreparedLibraryRootRemoval._(this._root, this._generation, this._publication);

  final LibraryRoot _root;
  final int _generation;
  final LibraryCatalogPublicationLease _publication;
}

class LibraryRootRemovalController {
  LibraryRootRemovalController(
    this._catalog,
    this._viewport,
    this._readState,
    this._publishState,
    this._isHostDisposed,
    this._hasUpdateForRoot,
    this._supersedeHostOperations,
  );

  final LibraryCatalog _catalog;
  final LibraryViewportController _viewport;
  final LibraryState Function() _readState;
  final void Function(LibraryState) _publishState;
  final bool Function() _isHostDisposed;
  final bool Function(String) _hasUpdateForRoot;
  final void Function() _supersedeHostOperations;

  int _generation = 0;
  PreparedLibraryRootRemoval? _preparedRemoval;
  LibraryCatalogPublicationLease? _publication;
  bool _isDisposed = false;

  LibraryState get _state => _readState();

  set _state(LibraryState value) => _publishState(value);

  PreparedLibraryRootRemoval? prepareRemoval(LibraryRoot root) {
    if (_isDisposed || _isHostDisposed()) {
      return null;
    }
    final currentRoot = _state.roots.where(
      (candidate) => candidate.id == root.id,
    );
    if (_state.isBusy ||
        _hasUpdateForRoot(root.id) ||
        currentRoot.length != 1 ||
        currentRoot.single.path != root.path) {
      return null;
    }

    final operation = _beginOperation();
    if (operation == null) {
      return null;
    }
    _state = _state.copyWith(
      status: LibraryStatus.removing,
      taskKind: LibraryTaskKind.remove,
      removingRootId: root.id,
      removingRootDisplayPath: root.displayPath,
      isRemovalCommitted: false,
      errorMessage: null,
    );
    final prepared = PreparedLibraryRootRemoval._(
      root,
      operation.generation,
      operation.publication,
    );
    _preparedRemoval = prepared;
    return prepared;
  }

  Future<bool> unregisterRoot(LibraryRoot root) async {
    final prepared = prepareRemoval(root);
    return prepared != null && await executeRemoval(prepared);
  }

  Future<bool> executeRemoval(PreparedLibraryRootRemoval prepared) async {
    final generation = prepared._generation;
    if (!_owns(generation) || !identical(_preparedRemoval, prepared)) {
      return false;
    }
    _preparedRemoval = null;
    final root = prepared._root;
    try {
      await _catalog.unregisterRoot(root.id);
      if (!_owns(generation)) {
        return false;
      }
      _viewport.publishCommittedRootRemovalProjection(root);
      final didReload = await _viewport.reloadCommittedRootRemovalFirstPage(
        removedRootId: root.id,
      );
      if (!_owns(generation) || !didReload) {
        return false;
      }
      _complete(root.id);
      return true;
    } on Object catch (error) {
      if (_owns(generation)) {
        _state = _state.copyWith(
          status: LibraryStatus.failed,
          errorMessage: error.toString(),
        );
      }
      return false;
    } finally {
      _releasePublication(prepared._publication);
    }
  }

  void abandonRemoval(PreparedLibraryRootRemoval prepared) {
    if (!_owns(prepared._generation) ||
        !identical(_preparedRemoval, prepared)) {
      return;
    }
    _preparedRemoval = null;
    _generation += 1;
    _state = _state.copyWith(
      status: _state.roots.isEmpty
          ? LibraryStatus.empty
          : LibraryStatus.completed,
      taskKind: null,
      removingRootId: null,
      removingRootDisplayPath: null,
      isRemovalCommitted: false,
      errorMessage: null,
    );
    _releasePublication(prepared._publication);
  }

  void dispose() {
    _isDisposed = true;
    _preparedRemoval = null;
    _generation += 1;
    final publication = _publication;
    if (publication != null) {
      _releasePublication(publication);
    }
  }

  Future<void> retryCommittedReload() async {
    final removedRootId = _state.removingRootId;
    if (!_state.isRemovalCommitted || removedRootId == null) {
      return;
    }
    final operation = _beginOperation();
    if (operation == null) {
      return;
    }
    final generation = operation.generation;
    _state = _state.copyWith(
      status: LibraryStatus.removing,
      errorMessage: null,
    );
    try {
      final didReload = await _viewport.reloadCommittedRootRemovalFirstPage(
        removedRootId: removedRootId,
      );
      if (!_owns(generation) || !didReload) {
        return;
      }
      _complete(removedRootId);
    } on Object catch (error) {
      if (_owns(generation)) {
        _state = _state.copyWith(
          status: LibraryStatus.failed,
          errorMessage: error.toString(),
        );
      }
    } finally {
      _releasePublication(operation.publication);
    }
  }

  ({int generation, LibraryCatalogPublicationLease publication})?
  _beginOperation() {
    final publication = _viewport.reserveCatalogPublication();
    if (publication == null) {
      return null;
    }
    _publication = publication;
    _supersedeHostOperations();
    return (generation: ++_generation, publication: publication);
  }

  void _releasePublication(LibraryCatalogPublicationLease publication) {
    if (identical(_publication, publication)) {
      _publication = null;
      _viewport.releaseCatalogPublication(publication);
    }
  }

  bool _owns(int generation) {
    return !_isDisposed && !_isHostDisposed() && generation == _generation;
  }

  void _complete(String removedRootId) {
    _state = _state.copyWith(
      status: _state.roots.isEmpty
          ? LibraryStatus.empty
          : LibraryStatus.completed,
      scanId: null,
      rootPath: null,
      displayRootPath: null,
      taskKind: null,
      removingRootId: null,
      removingRootDisplayPath: null,
      isRemovalCommitted: false,
      completedRemovalRootId: removedRootId,
      rootRemovalCompletionSequence: _state.rootRemovalCompletionSequence + 1,
      visitedEntries: 0,
      stagedAssetCount: 0,
      scanPhase: LibraryScanPhase.discovering,
      validatedAssetCount: 0,
      validationAssetCount: 0,
      itemLimit: null,
      entryLimit: null,
      isScanLimited: false,
      errorMessage: null,
    );
  }
}
