import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_organization_conflicts.dart";

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
    final colors = Theme.of(context).colorScheme;
    return IgnorePointer(
      child: Align(
        alignment: Alignment.bottomLeft,
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: Material(
            color: colors.surface.withValues(alpha: 0.92),
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              child: Text(
                "相册归属冲突",
                key: ValueKey("album-conflict-$locationId"),
                style: Theme.of(
                  context,
                ).textTheme.labelMedium?.copyWith(color: colors.onSurface),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
