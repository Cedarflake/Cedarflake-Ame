import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_settings.dart";
import "../../application/library_controller.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "library_classification_controls.dart";

class LibraryViewerClassification extends ConsumerWidget {
  const LibraryViewerClassification({required this.target, super.key});
  final LibraryOrganizationTarget target;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(libraryClassificationSettingsProvider).value;
    final revision = ref.watch(
      libraryControllerProvider.select((state) => state.catalogRevision),
    );
    if (revision == null || settings == null || settings.categories.isEmpty) {
      return const SizedBox.shrink();
    }
    return LibraryClassificationControls(
      key: ValueKey(target),
      viewer: true,
      enabled: true,
      scope: LibraryClassificationScope(
        query: const LibraryGalleryQuery(),
        catalogRevision: revision,
        allMatching: false,
        assetIds: [target.assetId],
        targets: [target],
      ),
    );
  }
}
