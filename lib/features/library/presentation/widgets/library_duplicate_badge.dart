import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_gallery_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_locations_flyout.dart";
import "library_gallery_card_metrics.dart";

class LibraryDuplicateBadge extends ConsumerStatefulWidget {
  const LibraryDuplicateBadge({required this.locationId, super.key});
  final String locationId;

  @override
  ConsumerState<LibraryDuplicateBadge> createState() =>
      _LibraryDuplicateBadgeState();
}

class _LibraryDuplicateBadgeState extends ConsumerState<LibraryDuplicateBadge> {
  final _menu = MenuController();
  final _focus = FocusNode();

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final groups = ref.watch(libraryGalleryDuplicateAnnotationsProvider);
    final group = groups.asData?.value[widget.locationId];
    if (group == null || !TickerMode.valuesOf(context).enabled) {
      return const SizedBox.shrink();
    }
    return AmeOverlayTraversalBoundary(
      child: MenuAnchor(
        controller: _menu,
        childFocusNode: _focus,
        consumeOutsideTap: true,
        useRootOverlay: true,
        style: const MenuStyle(
          padding: WidgetStatePropertyAll(EdgeInsets.zero),
          alignment: AlignmentDirectional.bottomStart,
        ),
        menuChildren: [
          LibraryDuplicateLocationsFlyout(
            group: group,
            currentLocationId: widget.locationId,
            onClose: _menu.close,
          ),
        ],
        child: AmeTooltip(
          message:
              "${LibraryDuplicateStrings.viewLocations} · ${LibraryDuplicateStrings.locations(group.locationCount)}",
          child: TextButton.icon(
            key: ValueKey("duplicate-badge-${widget.locationId}"),
            focusNode: _focus,
            style: TextButton.styleFrom(
              backgroundColor: Theme.of(
                context,
              ).colorScheme.surface.withValues(alpha: 0.92),
              foregroundColor: Theme.of(context).colorScheme.onSurface,
              minimumSize: const Size(40, 36),
              padding: const EdgeInsets.symmetric(
                horizontal:
                    LibraryGalleryCardMetrics.duplicateHorizontalPadding,
              ),
              textStyle: LibraryGalleryCardMetrics.badgeTextStyleOf(context),
            ),
            onPressed: () {
              if (_menu.isOpen) {
                _menu.close();
              } else {
                _menu.open();
              }
            },
            icon: const Icon(
              Symbols.photo_library_rounded,
              size: LibraryGalleryCardMetrics.duplicateIconExtent,
            ),
            label: Text("${group.locationCount}"),
          ),
        ),
      ),
    );
  }
}
