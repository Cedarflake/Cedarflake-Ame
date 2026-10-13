import "package:flutter_riverpod/flutter_riverpod.dart";

class LibraryScanExecutionCoordinator {
  Object? _primaryOwner;
  final Set<String> _updateRootIds = {};

  bool get hasPrimaryScan => _primaryOwner != null;

  bool get hasUpdateScans => _updateRootIds.isNotEmpty;

  bool get canStartPrimary => _primaryOwner == null && _updateRootIds.isEmpty;

  bool get canStartUpdates => _primaryOwner == null;

  bool tryAcquirePrimary(Object owner) {
    if (identical(_primaryOwner, owner)) {
      return true;
    }
    if (!canStartPrimary) {
      return false;
    }
    _primaryOwner = owner;
    return true;
  }

  bool hasUpdateForRoot(String rootId) => _updateRootIds.contains(rootId);

  bool tryAcquireUpdate(String rootId) {
    if (!canStartUpdates || rootId.isEmpty || _updateRootIds.contains(rootId)) {
      return false;
    }
    _updateRootIds.add(rootId);
    return true;
  }

  void releasePrimary(Object owner) {
    if (identical(_primaryOwner, owner)) {
      _primaryOwner = null;
    }
  }

  void releaseUpdate(String rootId) {
    _updateRootIds.remove(rootId);
  }
}

final libraryScanExecutionCoordinatorProvider =
    Provider<LibraryScanExecutionCoordinator>((ref) {
      return LibraryScanExecutionCoordinator();
    });
