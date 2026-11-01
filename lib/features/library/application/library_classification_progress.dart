import "package:flutter_riverpod/flutter_riverpod.dart";

import "../adapters/rust_library_classification_progress.dart";
import "../domain/library_classification_progress.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "library_classification_actions.dart";
import "library_controller.dart";

abstract interface class LibraryClassificationProgress {
  Future<LibraryClassificationPosition> begin(LibraryGalleryQuery query);
  Future<void> checkpoint(
    LibraryClassificationPosition position,
    LibraryOrganizationTarget target,
  );
  Future<void> end(String sessionId);
  Future<LibraryClassificationContinuation> continuation();
}

final libraryClassificationProgressProvider =
    Provider<LibraryClassificationProgress>(
      (ref) => const RustLibraryClassificationProgress(),
    );

final libraryClassificationContinuationProvider =
    FutureProvider.autoDispose<LibraryClassificationContinuation>((ref) {
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      ref.watch(
        libraryClassificationActionsProvider.select((state) => state.sequence),
      );
      return ref.read(libraryClassificationProgressProvider).continuation();
    });
