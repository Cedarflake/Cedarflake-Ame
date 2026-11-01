import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_retry.dart";
import "../../application/library_classification_retry_actions.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_review.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_controller.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";

class LibraryPhotoClassificationMenu {
  LibraryPhotoClassificationMenu._(this.service, this.target, this.settings);

  static LibraryPhotoClassificationMenu? forTarget(
    WidgetRef ref,
    LibraryOrganizationTarget target,
  ) {
    final state = ref.read(libraryControllerProvider);
    if (state.query.collection is! LibrarySmartAlbumCollection ||
        state.catalogRevision == null) {
      return null;
    }
    return LibraryPhotoClassificationMenu._(
      ref.read(libraryClassificationReviewServiceProvider),
      target,
      ref.read(libraryClassificationSettingsProvider).value,
    );
  }

  final LibraryClassificationReviewService service;
  final LibraryOrganizationTarget target;
  final LibraryClassificationSettings? settings;
  LibraryClassificationItem? _item;
  bool _disposed = false;

  bool get canRetry =>
      !_disposed && canRetryLibraryClassification(_item, settings);

  Future<void> prepare() async {
    final items = await service.loadItems([target]);
    if (!_disposed) _item = items.firstOrNull;
  }

  void dispose() => _disposed = true;

  Future<void> retry({
    required WidgetRef ref,
    required bool Function() isCurrent,
  }) async {
    final observed = settings;
    if (!isCurrent() || !canRetry || observed == null) return;
    await ref
        .read(libraryClassificationRetryActionsProvider.notifier)
        .retry(target, observed.dispatchRevision);
  }
}
