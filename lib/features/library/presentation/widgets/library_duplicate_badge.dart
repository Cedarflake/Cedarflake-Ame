import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_gallery_duplicates.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_locations_sheet.dart";
import "library_duplicate_update_status.dart";
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
    final group = groups.value?[widget.locationId];
    if (group == null || !TickerMode.valuesOf(context).enabled) {
      return const SizedBox.shrink();
    }
    final count = LibraryDuplicateStrings.locations(group.locationCount);
    return AmeOverlayTraversalBoundary(
      child: LibraryDuplicateUpdateStatus(
        hasPending: group.pendingLocationCount > 0,
        builder: (context, label) => AmeTooltip(
          message:
              "${LibraryDuplicateStrings.viewLocations} · $count${label == null ? '' : ' · $label'}",
          child: IconButton(
            key: ValueKey("duplicate-badge-${widget.locationId}"),
            focusNode: _focus,
            style: IconButton.styleFrom(
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
              final restoreFocus = _focus.hasFocus;
              await showModalBottomSheet<void>(
                context: context,
                showDragHandle: true,
                isScrollControlled: true,
                useSafeArea: true,
                builder: (_) => LibraryDuplicateLocationsSheet(
                  group: group,
                  currentLocationId: widget.locationId,
                ),
              );
              if (restoreFocus &&
                  context.mounted &&
                  TickerMode.valuesOf(context).enabled) {
                _focus.requestFocus();
              }
            },
            icon: Container(
              width: LibraryGalleryCardMetrics.controlExtent,
              height: LibraryGalleryCardMetrics.controlExtent,
              decoration: BoxDecoration(
                color: Theme.of(
                  context,
                ).colorScheme.surface.withValues(alpha: 0.92),
                shape: BoxShape.circle,
              ),
              alignment: Alignment.center,
              child: const Icon(
                Symbols.photo_library_rounded,
                size: LibraryGalleryCardMetrics.duplicateIconExtent,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
