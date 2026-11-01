import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_actions.dart";
import "../../application/library_classification_retry.dart";
import "../../application/library_classification_retry_actions.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_review.dart";
import "../../application/library_classification_selection.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_classification_progress.dart";
import "../../application/library_classification_review_session.dart";
import "../../application/library_controller.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "library_classification_category_menu.dart";

enum LibraryPhotoClassificationAction { confirm, correct, defer, retry }

class LibraryPhotoClassificationMenu {
  LibraryPhotoClassificationMenu._(
    this.selection,
    this.categories,
    this.progress,
    this.settings,
  );

  static LibraryPhotoClassificationMenu? forTarget(
    WidgetRef ref,
    LibraryOrganizationTarget target,
  ) {
    final state = ref.read(libraryControllerProvider);
    if (state.query.collection is! LibrarySmartAlbumCollection ||
        state.catalogRevision == null) {
      return null;
    }
    final categories = ref.read(libraryClassificationCategoriesProvider).value;
    if (categories == null || categories.isEmpty) {
      return null;
    }
    final progress = LibraryClassificationReviewSession(
      ref.read(libraryClassificationProgressProvider),
      state.query,
    );
    return LibraryPhotoClassificationMenu._(
      LibraryClassificationSelection(
        ref.read(libraryClassificationReviewServiceProvider),
        LibraryClassificationScope(
          query: state.query,
          catalogRevision: state.catalogRevision!,
          allMatching: false,
          assetIds: const [],
          targets: [target],
        ),
        reviewEntry: progress.open(target),
      ),
      List.unmodifiable(categories),
      progress,
      ref.read(libraryClassificationSettingsProvider).value,
    );
  }

  final LibraryClassificationSelection selection;
  final LibraryClassificationReviewSession progress;
  final List<LibraryClassificationCategory> categories;
  final LibraryClassificationSettings? settings;
  bool get canRetry =>
      canRetryLibraryClassification(selection.summary?.singleItem, settings);
  bool get isReady => selection.summary != null;
  bool get canConfirm => (selection.summary?.confirmable ?? 0) > 0;
  bool get canDefer => switch (selection.summary?.singleItem?.review) {
    null ||
    LibraryClassificationReview.confirmed ||
    LibraryClassificationReview.deferred => false,
    _ => true,
  };

  Future<void> prepare() async {
    await selection.prepare();
  }

  void dispose() {
    selection.dispose();
    unawaited(_closeProgress());
  }

  Future<void> _closeProgress() async {
    try {
      await progress.close();
    } on Object catch (error) {
      debugPrint("[Ame classification] context review close failed: $error");
    }
  }

  Future<void> apply({
    required LibraryPhotoClassificationAction action,
    required BuildContext context,
    required WidgetRef ref,
    required RelativeRect position,
    required bool Function() isCurrent,
  }) async {
    if (!isCurrent() || !isReady) return;
    if (action == LibraryPhotoClassificationAction.retry) {
      final observed = settings;
      if (!canRetry || observed == null) return;
      await ref
          .read(libraryClassificationRetryActionsProvider.notifier)
          .retry(selection.scope.targets.single, observed.dispatchRevision);
      return;
    }
    LibraryClassificationCommand command;
    if (action == LibraryPhotoClassificationAction.confirm) {
      if (!canConfirm) return;
      command = const LibraryConfirmClassification();
    } else if (action == LibraryPhotoClassificationAction.defer) {
      if (!canDefer) return;
      command = const LibraryDeferClassification();
    } else {
      final choice = await showLibraryClassificationCategoryMenu(
        context: context,
        position: position,
        categories: categories,
        selectedCategoryId: selection.summary?.commonCategoryId,
      );
      if (!context.mounted || !isCurrent() || choice == null) return;
      command = LibraryCorrectClassification(choice.substring(9));
    }
    await ref
        .read(libraryClassificationActionsProvider.notifier)
        .apply(selection, command);
  }
}
