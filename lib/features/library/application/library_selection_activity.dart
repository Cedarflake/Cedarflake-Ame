import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_organization_batch.dart";

class LibrarySelectionActivity {
  const LibrarySelectionActivity(this.owner, {this.progress});

  final Object owner;
  final LibraryBatchProgress? progress;
}

class LibrarySelectionActivityController
    extends Notifier<LibrarySelectionActivity?> {
  @override
  LibrarySelectionActivity? build() => null;

  void start(Object owner) => state = LibrarySelectionActivity(owner);

  void update(Object owner, LibraryBatchProgress progress) {
    if (identical(state?.owner, owner)) {
      state = LibrarySelectionActivity(owner, progress: progress);
    }
  }

  void finish(Object owner) {
    if (!ref.mounted) return;
    if (identical(state?.owner, owner)) state = null;
  }
}

final librarySelectionActivityProvider =
    NotifierProvider<
      LibrarySelectionActivityController,
      LibrarySelectionActivity?
    >(LibrarySelectionActivityController.new);
