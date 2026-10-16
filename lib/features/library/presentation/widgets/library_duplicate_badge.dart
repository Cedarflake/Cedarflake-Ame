import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_gallery_duplicates.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_locations_dialog.dart";
import "library_gallery_card_metrics.dart";

class LibraryDuplicateBadge extends ConsumerStatefulWidget {
  const LibraryDuplicateBadge({required this.locationId, super.key});
  final String locationId;

  @override
  ConsumerState<LibraryDuplicateBadge> createState() =>
      _LibraryDuplicateBadgeState();
}

class _LibraryDuplicateBadgeState extends ConsumerState<LibraryDuplicateBadge> {
  final _focus = FocusNode();

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final mode = ref.watch(
      libraryControllerProvider.select((state) => state.query.duplicateMode),
    );
    if (mode != LibraryGalleryDuplicateMode.merged) {
      return const SizedBox.shrink();
    }
    final groups = ref.watch(libraryGalleryDuplicateAnnotationsProvider);
    final group = groups.asData?.value[widget.locationId];
    if (group == null || !TickerMode.valuesOf(context).enabled) {
      return const SizedBox.shrink();
    }
    return AmeOverlayTraversalBoundary(
      child: AmeTooltip(
        message:
            "${LibraryDuplicateStrings.viewLocations} · ${LibraryDuplicateStrings.locations(group.locationCount)}",
        child: IconButton(
          key: ValueKey("duplicate-badge-${widget.locationId}"),
          focusNode: _focus,
          style: IconButton.styleFrom(
            backgroundColor: Theme.of(
              context,
            ).colorScheme.surface.withValues(alpha: 0.92),
            foregroundColor: Theme.of(context).colorScheme.onSurface,
            fixedSize: const Size.square(
              LibraryGalleryCardMetrics.selectionExtent,
            ),
            shape: const CircleBorder(),
            padding: EdgeInsets.zero,
            visualDensity: VisualDensity.standard,
            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
          onPressed: () async {
            await showDialog<void>(
              context: context,
              builder: (_) => LibraryDuplicateLocationsDialog(
                group: group,
                currentLocationId: widget.locationId,
              ),
            );
            if (context.mounted && TickerMode.valuesOf(context).enabled) {
              _focus.requestFocus();
            }
          },
          icon: const Icon(
            Symbols.photo_library_rounded,
            size: LibraryGalleryCardMetrics.duplicateIconExtent,
          ),
        ),
      ),
    );
  }
}
