import "../domain/library_state.dart";

class LibraryQueryTransitionRead {
  const LibraryQueryTransitionRead._(this.generation, this.baseState);

  final int generation;
  final LibraryState baseState;
}

/// Retains the last published state across consecutive query replacements.
/// Only the active read can release its baseline and loading ownership.
class LibraryQueryTransition {
  LibraryQueryTransitionRead? _active;

  LibraryQueryTransitionRead? currentFor(int generation) {
    final active = _active;
    return active?.generation == generation ? active : null;
  }

  LibraryQueryTransitionRead begin({
    required int generation,
    required int previousGeneration,
    required LibraryState currentState,
  }) {
    final baseState = currentFor(previousGeneration)?.baseState ?? currentState;
    final read = LibraryQueryTransitionRead._(generation, baseState);
    _active = read;
    return read;
  }

  bool finish(LibraryQueryTransitionRead read) {
    if (!identical(_active, read)) {
      return false;
    }
    _active = null;
    return true;
  }

  bool retire() {
    final hadActiveRead = _active != null;
    _active = null;
    return hadActiveRead;
  }
}
