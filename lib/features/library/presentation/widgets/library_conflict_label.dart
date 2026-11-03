import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_organization_conflicts.dart";
import "library_photo_status_label.dart";

class LibraryConflictLabel extends ConsumerWidget {
  const LibraryConflictLabel({required this.locationId, super.key});
  final String locationId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final conflicts = ref
        .watch(libraryConflictAnnotationsProvider)
        .asData
        ?.value;
    if (conflicts?.contains(locationId) != true) return const SizedBox.shrink();
    return IgnorePointer(
      child: LibraryPhotoStatusLabel(
        label: "相册归属冲突",
        textKey: ValueKey("album-conflict-$locationId"),
      ),
    );
  }
}
