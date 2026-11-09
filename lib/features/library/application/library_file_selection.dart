import "../domain/library_file_operations.dart";
import "ports/library_file_operations.dart";

class LibraryFileSelectionSession {
  LibraryFileSelectionSession(this.service, this.selection);
  final LibraryFileOperations service;
  final LibraryFileSelection selection;
  LibraryFileSelectionDraft? _draft;
  Future<LibraryFileSelectionSummary>? _preparing;
  Future<void>? _closing;
  bool _cancelled = false;

  void _check() {
    if (_cancelled) {
      throw const LibraryFileFailure(
        "file_operation_selection_closed",
        "Selection was retired",
      );
    }
  }

  Future<LibraryFileSelectionSummary> prepare() => _preparing ??= _prepare();

  Future<LibraryFileSelectionSummary> _prepare() async {
    _check();
    final draft = await service.open(selection);
    _draft = draft;
    if (_cancelled) {
      await draft.close();
      _check();
    }
    switch (selection) {
      case LibraryFilesInGallery(:final selection):
        for (var start = 0; start < selection.assetIds.length; start += 256) {
          _check();
          await draft.appendIds(
            selection.assetIds.sublist(
              start,
              (start + 256).clamp(0, selection.assetIds.length),
            ),
          );
        }
      case LibraryFilesAtLocations(:final locations):
        for (var start = 0; start < locations.length; start += 256) {
          _check();
          await draft.appendLocations(
            locations.sublist(start, (start + 256).clamp(0, locations.length)),
          );
        }
    }
    _check();
    return draft.prepare();
  }

  Future<LibraryFileTask> execute(
    String batchId,
    LibraryFileCommand command,
  ) async {
    await prepare();
    _check();
    return _draft!.execute(batchId, command);
  }

  Future<LibraryFileProgress?> progress() async => _draft?.progress();

  Future<void> cancel() async {
    _cancelled = true;
    await _draft?.cancel();
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    await cancel();
    try {
      await _preparing;
    } on Object {
      /* Preparation owns its failure. */
    }
    await _draft?.close();
  }
}
