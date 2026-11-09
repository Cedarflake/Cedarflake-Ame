import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_file_operations.dart";
import "library_file_selection.dart";
import "library_organization_publication.dart";
import "library_organization_requests.dart";
import "ports/library_file_operations.dart";

final libraryFileOperationsProvider = Provider<LibraryFileOperations>((ref) {
  throw StateError("LibraryFileOperations must be bound during startup");
});

class LibraryFileActivity {
  const LibraryFileActivity({
    this.action,
    this.progress,
    this.stopping = false,
    this.recovering = false,
  });
  final LibraryFileAction? action;
  final LibraryFileProgress? progress;
  final bool stopping;
  final bool recovering;
  bool get busy => progress != null && !progress!.isFinished;
}

final libraryFileActionsProvider =
    NotifierProvider<LibraryFileActions, LibraryFileActivity>(
      LibraryFileActions.new,
    );

class LibraryFileActions extends Notifier<LibraryFileActivity> {
  LibraryFileSelectionSession? _selection;
  LibraryFileTask? _task;
  Future<void>? _running;
  Future<void>? _refreshing;
  bool _closing = false;
  bool _disposed = false;
  bool _stopping = false;
  int _published = 0;

  @override
  LibraryFileActivity build() {
    ref.onDispose(() {
      _disposed = true;
      unawaited(close());
    });
    return const LibraryFileActivity();
  }

  Future<void> execute(
    LibraryFileSelectionSession selection,
    LibraryFileCommand command,
  ) {
    if (_closing || _running != null) {
      return selection.close();
    }
    _selection = selection;
    _stopping = false;
    _published = 0;
    state = LibraryFileActivity(
      action: command.action,
      progress: const LibraryFileProgress(
        phase: LibraryFileTaskPhase.preparing,
      ),
    );
    return _running = _run(
      () => selection.execute(newOrganizationOperationId(), command),
    );
  }

  Future<void> recover(List<BigInt> ids, {bool continueMove = false}) {
    if (_closing || _running != null) return Future.value();
    _stopping = false;
    _published = 0;
    state = const LibraryFileActivity(
      recovering: true,
      progress: LibraryFileProgress(phase: LibraryFileTaskPhase.preparing),
    );
    final service = ref.read(libraryFileOperationsProvider);
    return _running = _run(
      () => service.recover(ids, continueMove: continueMove),
    );
  }

  Future<void> _run(Future<LibraryFileTask> Function() start) async {
    Timer? preparationTimer;
    LibraryFileProgress? completed;
    var readingPreparation = false;
    preparationTimer = Timer.periodic(const Duration(milliseconds: 250), (
      _,
    ) async {
      if (readingPreparation || _selection == null || _task != null) return;
      readingPreparation = true;
      try {
        final progress = await _selection?.progress();
        if (preparationTimer?.isActive == true &&
            _task == null &&
            progress != null) {
          _accept(progress);
        }
      } on Object {
        // The preparation result owns the error and retires its unstarted records.
      } finally {
        readingPreparation = false;
      }
    });
    try {
      try {
        _task = await start();
        preparationTimer.cancel();
        if (_stopping) await _task!.cancel();
        while (true) {
          final progress = await _task!.progress();
          if (progress.isFinished) {
            completed = progress;
          } else {
            _accept(progress);
          }
          if (progress.published > _published) _refresh(progress.published);
          if (progress.isFinished) break;
          await Future<void>.delayed(const Duration(milliseconds: 200));
        }
      } finally {
        preparationTimer.cancel();
        await _task?.close();
        _task = null;
        await _selection?.close();
        _selection = null;
      }
      if (completed != null && completed.published > 0) {
        _accept(completed.withPhase(LibraryFileTaskPhase.refreshing));
      }
      if (!_disposed) {
        await _refreshing;
        if ((state.progress?.published ?? 0) > _published) {
          await ref.read(libraryOrganizationPublicationProvider).refresh();
        }
      }
      if (completed != null) _accept(completed);
    } on Object catch (error) {
      final previous = completed ?? (_disposed ? null : state.progress);
      final cancelledPreparation =
          _stopping &&
          error is LibraryFileFailure &&
          error.code == "file_operation_selection_closed";
      final failure = error is LibraryFileFailure
          ? error
          : LibraryFileFailure("file_operation_failed", error.toString());
      _accept(
        LibraryFileProgress(
          phase: cancelledPreparation
              ? LibraryFileTaskPhase.cancelled
              : LibraryFileTaskPhase.needsAttention,
          total: previous?.total ?? 0,
          processed: previous?.processed ?? 0,
          published: previous?.published ?? 0,
          failed: previous?.failed ?? 0,
          cancelled: previous?.cancelled ?? 0,
          failure: cancelledPreparation ? null : failure,
        ),
      );
    } finally {
      _task = null;
      _selection = null;
      _running = null;
    }
  }

  void _accept(LibraryFileProgress progress) {
    if (_disposed) return;
    state = LibraryFileActivity(
      action: state.action,
      recovering: state.recovering,
      progress: progress,
      stopping: _stopping && !progress.isFinished,
    );
  }

  void _refresh(int published) {
    if (_disposed || _refreshing != null) return;
    _published = published;
    _refreshing = ref
        .read(libraryOrganizationPublicationProvider)
        .refresh()
        .whenComplete(() => _refreshing = null);
  }

  Future<void> cancel() async {
    if (_running == null) return;
    _stopping = true;
    if (!_disposed && state.progress != null) _accept(state.progress!);
    await _task?.cancel();
    await _selection?.cancel();
  }

  void dismiss() {
    if (_running == null && !_disposed) state = const LibraryFileActivity();
  }

  Future<void> close() async {
    _closing = true;
    await cancel();
    await _running;
    await _refreshing;
  }
}
