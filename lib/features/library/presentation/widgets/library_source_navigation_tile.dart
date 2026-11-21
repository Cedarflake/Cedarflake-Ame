import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_popup_menu_position.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../../app/presentation/ame_transition_icon.dart";
import "../../domain/library_models.dart";
import "../../domain/library_synchronization_models.dart";
import "../library_pending_source.dart";
import "../library_synchronization_feedback.dart";
import "library_path_text.dart";
import "library_sidebar_selection_icon.dart";
import "library_workbench_metrics.dart";

class LibrarySourceNavigationTile extends StatefulWidget {
  const LibrarySourceNavigationTile({
    required this.root,
    this.synchronizationStatus,
    this.hasSynchronizationFailure = false,
    required this.isCompact,
    required this.isSelected,
    required this.isExpanded,
    required this.isBrowseDisabled,
    this.isUpdateDisabled = false,
    this.isUpdating = false,
    required this.onSelect,
    required this.onToggleExpansion,
    required this.onUpdate,
    this.onRelocate,
    required this.onOpen,
    required this.onRemove,
    this.focusNode,
    this.checked = false,
    this.onToggleSelection,
    super.key,
  });

  final LibraryRoot root;
  final LibraryRootSynchronizationStatus? synchronizationStatus;
  final bool hasSynchronizationFailure;
  final bool isCompact;
  final bool isSelected;
  final bool isExpanded;
  final bool isBrowseDisabled;
  final bool isUpdateDisabled;
  final bool isUpdating;
  final VoidCallback onSelect;
  final VoidCallback? onToggleExpansion;
  final VoidCallback onUpdate;
  final VoidCallback? onRelocate;
  final VoidCallback onOpen;
  final VoidCallback onRemove;
  final FocusNode? focusNode;
  final bool checked;
  final VoidCallback? onToggleSelection;

  @override
  State<LibrarySourceNavigationTile> createState() =>
      _LibrarySourceNavigationTileState();
}

class _LibrarySourceNavigationTileState
    extends State<LibrarySourceNavigationTile> {
  late FocusNode _focusNode;
  late bool _ownsFocusNode;
  final GlobalKey _moreButtonAnchorKey = GlobalKey(
    debugLabel: "Library source more button",
  );

  @override
  void initState() {
    super.initState();
    _focusNode = widget.focusNode ?? FocusNode(debugLabel: "Library source");
    _ownsFocusNode = widget.focusNode == null;
  }

  @override
  void didUpdateWidget(covariant LibrarySourceNavigationTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (identical(oldWidget.focusNode, widget.focusNode)) {
      return;
    }
    if (_ownsFocusNode) {
      _focusNode.dispose();
    }
    _focusNode = widget.focusNode ?? FocusNode(debugLabel: "Library source");
    _ownsFocusNode = widget.focusNode == null;
  }

  @override
  void dispose() {
    if (_ownsFocusNode) {
      _focusNode.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final icon = switch (widget.root.availability) {
      LibraryRootAvailability.available || LibraryRootAvailability.unknown =>
        widget.isExpanded
            ? Symbols.folder_open_rounded
            : Symbols.folder_rounded,
      LibraryRootAvailability.missing => Symbols.folder_off_rounded,
      LibraryRootAvailability.inaccessible => Symbols.lock_rounded,
      LibraryRootAvailability.offline => Symbols.cloud_off_rounded,
    };
    final iconWidget = switch (widget.root.availability) {
      LibraryRootAvailability.available ||
      LibraryRootAvailability.unknown => AmeCrossFadeIcon(
        active: widget.isExpanded,
        inactiveIcon: Symbols.folder_rounded,
        activeIcon: Symbols.folder_open_rounded,
      ),
      _ => Icon(icon),
    };
    final statusLabel = _statusLabel(
      context.l10n,
      widget.root,
      widget.synchronizationStatus,
      widget.hasSynchronizationFailure,
      widget.isUpdating,
    );
    final expandedSubtitle =
        !widget.isUpdating &&
            _isHealthyLiveOnlySynchronization(
              widget.root,
              widget.synchronizationStatus,
              widget.hasSynchronizationFailure,
            )
        ? null
        : statusLabel;
    final tile = widget.isCompact
        ? AmeTooltip(
            message: [widget.root.displayPath, statusLabel].join("\n"),
            child: IconButton(
              focusNode: _focusNode,
              isSelected: widget.isSelected,
              onPressed: widget.isBrowseDisabled ? null : widget.onSelect,
              icon: iconWidget,
            ),
          )
        : _ExpandedSourceTile(
            focusNode: _focusNode,
            icon: icon,
            leading: LibrarySidebarSelectionIcon(
              key: ValueKey("library-sidebar-select-root-${widget.root.id}"),
              icon: iconWidget,
              name: librarySourceName(widget.root.displayPath),
              selected: widget.checked,
              onPressed: widget.onToggleSelection,
            ),
            iconKey: ValueKey("source-icon-${widget.root.id}"),
            title: librarySourceName(widget.root.displayPath),
            path: widget.root.displayPath,
            titleKey: ValueKey("source-title-${widget.root.id}"),
            subtitle: expandedSubtitle,
            isSelected: widget.isSelected,
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (widget.onToggleExpansion != null)
                  AmeTooltip(
                    message: widget.isExpanded
                        ? context.l10n.collapseFolder
                        : context.l10n.expandFolder,
                    child: IconButton(
                      key: ValueKey("source-expand-${widget.root.id}"),
                      onPressed: widget.onToggleExpansion,
                      icon: AmeExpansionIcon(
                        expanded: widget.isExpanded,
                        icon: Symbols.keyboard_arrow_down_rounded,
                      ),
                    ),
                  ),
                AmeTooltip(
                  message: context.l10n.more,
                  child: SizedBox(
                    key: _moreButtonAnchorKey,
                    child: IconButton(
                      key: ValueKey("source-more-${widget.root.id}"),
                      onPressed: () {
                        final anchorContext =
                            _moreButtonAnchorKey.currentContext;
                        if (anchorContext != null) {
                          unawaited(_showMenuAtAnchor(anchorContext));
                        }
                      },
                      icon: const Icon(Symbols.more_vert_rounded),
                    ),
                  ),
                ),
              ],
            ),
            onTap: widget.isBrowseDisabled ? null : widget.onSelect,
          );
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.contextMenu): () {
          final anchorContext = _focusNode.context;
          if (anchorContext != null) {
            unawaited(_showMenuAtAnchor(anchorContext));
          }
        },
        const SingleActivator(LogicalKeyboardKey.f10, shift: true): () {
          final anchorContext = _focusNode.context;
          if (anchorContext != null) {
            unawaited(_showMenuAtAnchor(anchorContext));
          }
        },
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onSecondaryTapDown: (details) {
          unawaited(_showMenuAtGlobalPosition(details.globalPosition));
        },
        child: tile,
      ),
    );
  }

  Future<void> _showMenuAtAnchor(BuildContext anchorContext) async {
    final position = amePopupMenuBelowAnchor(
      context: context,
      anchorContext: anchorContext,
    );
    if (position != null) {
      await _showMenu(position);
    }
  }

  Future<void> _showMenuAtGlobalPosition(Offset globalPosition) async {
    _focusNode.requestFocus();
    final position = amePopupMenuAtGlobalPosition(
      context: context,
      globalPosition: globalPosition,
    );
    if (position != null) {
      await _showMenu(position);
    }
  }

  Future<void> _showMenu(RelativeRect position) async {
    final labels = [
      context.l10n.updateLibrary,
      context.l10n.relocateLibrary,
      context.l10n.openInExplorer,
      context.l10n.removeFromAme,
    ];
    final action = await showAmePopupMenu<_LibrarySourceMenuAction>(
      context: context,
      position: position,
      labels: labels,
      items: [
        PopupMenuItem(
          value: _LibrarySourceMenuAction.update,
          enabled:
              !widget.isBrowseDisabled &&
              !widget.isUpdating &&
              !widget.isUpdateDisabled,
          child: AmeMenuItemContent(
            icon: Symbols.refresh_rounded,
            label: context.l10n.updateLibrary,
          ),
        ),
        PopupMenuItem(
          value: _LibrarySourceMenuAction.open,
          child: AmeMenuItemContent(
            icon: Symbols.folder_open_rounded,
            label: context.l10n.openInExplorer,
          ),
        ),
        PopupMenuItem(
          value: _LibrarySourceMenuAction.relocate,
          enabled:
              !widget.isBrowseDisabled &&
              !widget.isUpdating &&
              widget.onRelocate != null,
          child: AmeMenuItemContent(
            icon: Symbols.folder_open_rounded,
            label: context.l10n.relocateLibrary,
          ),
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        PopupMenuItem(
          value: _LibrarySourceMenuAction.remove,
          enabled: !widget.isBrowseDisabled && !widget.isUpdating,
          child: AmeMenuItemContent(
            icon: Symbols.remove_circle_rounded,
            label: context.l10n.removeFromAme,
          ),
        ),
      ],
    );
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case _LibrarySourceMenuAction.update:
        widget.onUpdate();
      case _LibrarySourceMenuAction.relocate:
        widget.onRelocate?.call();
      case _LibrarySourceMenuAction.open:
        widget.onOpen();
      case _LibrarySourceMenuAction.remove:
        widget.onRemove();
    }
  }

  static String _statusLabel(
    AppLocalizations strings,
    LibraryRoot root,
    LibraryRootSynchronizationStatus? synchronizationStatus,
    bool hasSynchronizationFailure,
    bool isUpdating,
  ) {
    if (synchronizationStatus?.isAwaitingFirstImport == true) {
      return strings.firstImportIncomplete;
    }
    if (root.availability != LibraryRootAvailability.available) {
      return switch (root.availability) {
        LibraryRootAvailability.available => strings.sourceAvailable,
        LibraryRootAvailability.missing => strings.sourceMissing,
        LibraryRootAvailability.inaccessible => strings.sourceUnavailable,
        LibraryRootAvailability.offline => strings.sourceOffline,
        LibraryRootAvailability.unknown => strings.sourceUnknown,
      };
    }
    if (synchronizationStatus case final status?
        when status.availability != LibraryRootAvailability.available) {
      return switch (status.availability) {
        LibraryRootAvailability.available => strings.sourceAvailable,
        LibraryRootAvailability.missing => strings.sourceMissing,
        LibraryRootAvailability.inaccessible => strings.sourceUnavailable,
        LibraryRootAvailability.offline => strings.sourceOffline,
        LibraryRootAvailability.unknown => strings.sourceUnknown,
      };
    }
    if (synchronizationStatus?.freshness ==
        LibraryCatalogFreshness.unavailable) {
      return strings.sourceUnavailable;
    }
    if (synchronizationStatus?.continuity ==
        LibraryContinuityState.unavailable) {
      return strings.continuityUnavailable;
    }
    if (isUpdating) {
      return strings.synchronizing;
    }
    if (hasSynchronizationFailure) {
      return strings.needsReconciliation;
    }
    if (synchronizationStatus == null) {
      return strings.synchronizationUnconfirmed;
    }
    return LibrarySynchronizationFeedback.fromStatus(
      synchronizationStatus,
    ).sourceLabel.resolve(strings);
  }

  static bool _isHealthyLiveOnlySynchronization(
    LibraryRoot root,
    LibraryRootSynchronizationStatus? synchronizationStatus,
    bool hasSynchronizationFailure,
  ) {
    return !hasSynchronizationFailure &&
        root.availability == LibraryRootAvailability.available &&
        synchronizationStatus?.availability ==
            LibraryRootAvailability.available &&
        synchronizationStatus?.freshness ==
            LibraryCatalogFreshness.synchronized &&
        synchronizationStatus?.freshnessCause ==
            LibraryCatalogFreshnessCause.noPendingChanges &&
        synchronizationStatus?.continuity == LibraryContinuityState.liveOnly &&
        synchronizationStatus?.usesAutomaticMetadataFallback == false &&
        synchronizationStatus?.phase ==
            LibrarySynchronizationPhase.synchronized &&
        synchronizationStatus?.sourceStatus ==
            LibraryChangeSourceStatus.healthy &&
        synchronizationStatus?.pendingChangeCount == BigInt.zero &&
        synchronizationStatus?.retryWaitCount == BigInt.zero &&
        synchronizationStatus?.freshnessUnknownCount == BigInt.zero &&
        synchronizationStatus?.recoveryBlocked == false;
  }
}

enum _LibrarySourceMenuAction { update, open, relocate, remove }

class PendingLibrarySourceTile extends StatelessWidget {
  const PendingLibrarySourceTile({
    required this.source,
    required this.isCompact,
    super.key,
  });

  final LibraryPendingSource source;
  final bool isCompact;

  @override
  Widget build(BuildContext context) {
    if (isCompact) {
      return AmeTooltip(
        message: "${source.path}\n${source.statusLabel(context.l10n)}",
        child: IconButton(
          onPressed: null,
          icon: Stack(
            clipBehavior: Clip.none,
            children: [
              const Icon(Symbols.folder_rounded),
              if (source.isProcessing)
                const Positioned(
                  right: -5,
                  bottom: -5,
                  child: SizedBox.square(
                    dimension: 12,
                    child: AmeCircularProgress(strokeWidth: 2),
                  ),
                ),
            ],
          ),
        ),
      );
    }
    return _ExpandedSourceTile(
      key: const Key("pending-source-tile"),
      icon: Symbols.folder_rounded,
      iconKey: const Key("pending-source-icon"),
      title: librarySourceName(source.path),
      path: source.path,
      titleKey: const Key("pending-source-title"),
      subtitle: source.statusLabel(context.l10n),
      trailing: SizedBox(
        width: 40,
        child: source.isProcessing
            ? const Align(
                alignment: Alignment.centerRight,
                child: SizedBox(
                  width: 40,
                  child: Center(
                    child: SizedBox.square(
                      key: Key("pending-source-progress"),
                      dimension: 18,
                      child: AmeCircularProgress(strokeWidth: 2.5),
                    ),
                  ),
                ),
              )
            : null,
      ),
    );
  }
}

String librarySourceName(String path) {
  final segments = path
      .split("\\")
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return segments.isEmpty ? path : segments.last;
}

class _ExpandedSourceTile extends StatelessWidget {
  const _ExpandedSourceTile({
    required this.icon,
    required this.title,
    required this.path,
    required this.trailing,
    this.iconKey,
    this.leading,
    this.titleKey,
    this.subtitle,
    this.isSelected = false,
    this.focusNode,
    this.onTap,
    super.key,
  });

  final IconData icon;
  final Key? iconKey;
  final Widget? leading;
  final String title;
  final String path;
  final Key? titleKey;
  final String? subtitle;
  final bool isSelected;
  final Widget trailing;
  final FocusNode? focusNode;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      focusNode: focusNode,
      selected: isSelected,
      selectedTileColor: AmeWorkbenchSurfaces.of(context).selection,
      selectedColor: AmeWorkbenchSurfaces.of(context).onSelection,
      shape: LibraryWorkbenchMetrics.sidebarRowShape,
      minTileHeight: LibraryWorkbenchMetrics.sidebarRowHeight,
      minVerticalPadding: 4,
      contentPadding: EdgeInsets.only(left: leading == null ? 16 : 8, right: 4),
      minLeadingWidth: leading == null ? 24 : 40,
      horizontalTitleGap: leading == null ? 12 : 4,
      leading: SizedBox(
        key: iconKey,
        width: leading == null ? 24 : 40,
        child: leading ?? Icon(icon),
      ),
      title: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          DefaultTextStyle.merge(
            style: TextStyle(
              fontWeight: isSelected
                  ? ameFontWeightSemibold
                  : ameFontWeightRegular,
            ),
            child: LibraryPathText(text: title, path: path, textKey: titleKey),
          ),
          if (subtitle != null)
            Text(
              subtitle!,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: isSelected
                    ? AmeWorkbenchSurfaces.of(context).onSelection
                    : Theme.of(context).colorScheme.onSurfaceVariant,
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
        ],
      ),
      trailing: trailing,
      onTap: onTap,
    );
  }
}
