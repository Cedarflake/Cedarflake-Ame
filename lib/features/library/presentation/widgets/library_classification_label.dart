import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_annotations.dart";
import "library_photo_status_label.dart";

class LibraryClassificationLabel extends ConsumerWidget {
  const LibraryClassificationLabel({required this.locationId, super.key});
  final String locationId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final reason = ref
        .watch(libraryClassificationAnnotationsProvider)
        .asData
        ?.value[locationId];
    if (reason == null || !reason.showsIndicator) {
      return const SizedBox.shrink();
    }
    return LibraryPhotoStatusLabel(
      label: reason.label,
      textKey: ValueKey("classification-review-$locationId"),
      detail: reason.description,
    );
  }
}
