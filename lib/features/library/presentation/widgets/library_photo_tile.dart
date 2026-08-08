import "dart:io";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/ame_menu.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_loading_indicator.dart";

class LibraryPhotoTile extends ConsumerStatefulWidget {
  const LibraryPhotoTile({
    required this.asset,
    required this.width,
    required this.height,
    required this.isSelecting,
    required this.isSelected,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    super.key,
  });

  final LibraryAsset asset;
  final double width;
  final double height;
  final bool isSelecting;
  final bool isSelected;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;

  @override
  ConsumerState<LibraryPhotoTile> createState() => _LibraryPhotoTileState();
}

class _LibraryPhotoTileState extends ConsumerState<LibraryPhotoTile> {
  final MenuController _menuController = MenuController();
  final FocusNode _focusNode = FocusNode(debugLabel: "Library photo tile");
  late final LibraryController _controller;
  bool _isHovered = false;
  bool _isFocused = false;

  @override
  void initState() {
    super.initState();
    _controller = ref.read(libraryControllerProvider.notifier);
    _schedulePreview();
  }

  @override
  void didUpdateWidget(covariant LibraryPhotoTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId ||
        oldWidget.asset.previewStatus != widget.asset.previewStatus) {
      _controller.cancelPreview(oldWidget.asset.locationId);
      _schedulePreview();
    }
  }

  @override
  void dispose() {
    _controller.cancelPreview(widget.asset.locationId);
    _focusNode.dispose();
    super.dispose();
  }

  void _schedulePreview() {
    if (widget.asset.previewStatus != LibraryPreviewStatus.pending) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _controller.requestPreview(widget.asset);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isSelectionVisible =
        _isHovered || _isFocused || widget.isSelecting || widget.isSelected;
    return SizedBox(
      width: widget.width,
      height: widget.height,
      child: AmeMenuAnchor(
        controller: _menuController,
        childFocusNode: _focusNode,
        menuChildren: [
          MenuItemButton(
            onPressed: () => widget.onOpen(widget.asset),
            child: const AmeMenuItemContent(
              icon: Icons.open_in_full,
              label: LibraryStrings.open,
            ),
          ),
          MenuItemButton(
            onPressed: () => widget.onViewInformation(widget.asset),
            child: const AmeMenuItemContent(
              icon: Icons.info_outline,
              label: LibraryStrings.viewInformation,
            ),
          ),
          const Divider(height: AmeMenuMetrics.dividerHeight),
          MenuItemButton(
            onPressed: () => widget.onCopyPath(widget.asset),
            child: const AmeMenuItemContent(
              icon: Icons.content_copy_outlined,
              label: LibraryStrings.copyPath,
            ),
          ),
          MenuItemButton(
            onPressed: () => widget.onRevealFile(widget.asset),
            child: const AmeMenuItemContent(
              icon: Icons.folder_open_outlined,
              label: LibraryStrings.openInExplorer,
            ),
          ),
        ],
        child: CallbackShortcuts(
          bindings: {
            const SingleActivator(LogicalKeyboardKey.contextMenu):
                _openKeyboardMenu,
            const SingleActivator(LogicalKeyboardKey.f10, shift: true):
                _openKeyboardMenu,
          },
          child: Focus(
            focusNode: _focusNode,
            onFocusChange: (value) => setState(() => _isFocused = value),
            child: MouseRegion(
              onEnter: (_) => setState(() => _isHovered = true),
              onExit: (_) => setState(() => _isHovered = false),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onSecondaryTapDown: (details) {
                  _focusNode.requestFocus();
                  _menuController.open(position: details.localPosition);
                },
                child: Semantics(
                  label: widget.asset.relativePath,
                  selected: widget.isSelected,
                  button: true,
                  child: Material(
                    color: colorScheme.surfaceContainerHighest,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(10),
                      side: widget.isSelected
                          ? BorderSide(color: colorScheme.primary, width: 3)
                          : BorderSide.none,
                    ),
                    clipBehavior: Clip.antiAlias,
                    child: InkWell(
                      onTap: () => widget.onOpen(widget.asset),
                      child: Stack(
                        fit: StackFit.expand,
                        children: [
                          _buildPreview(context),
                          if (isSelectionVisible)
                            Positioned(
                              right: 4,
                              top: 4,
                              child: Material(
                                color: colorScheme.surface.withValues(
                                  alpha: 0.92,
                                ),
                                shape: RoundedRectangleBorder(
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Checkbox(
                                  key: ValueKey(
                                    "select-${widget.asset.locationId}",
                                  ),
                                  value: widget.isSelected,
                                  onChanged: (_) =>
                                      widget.onToggleSelection(widget.asset),
                                  materialTapTargetSize:
                                      MaterialTapTargetSize.shrinkWrap,
                                  visualDensity: VisualDensity.compact,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildPreview(BuildContext context) {
    final asset = widget.asset;
    return switch (asset.previewStatus) {
      LibraryPreviewStatus.pending => const LibraryLoadingIndicator(
        key: Key("library-preview-pending"),
      ),
      LibraryPreviewStatus.failed => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.broken_image_outlined),
            const SizedBox(height: 4),
            TextButton(
              key: Key("preview-retry-${asset.locationId}"),
              onPressed: () => _controller.requestPreview(asset, retry: true),
              child: const Text(LibraryStrings.retryPreview),
            ),
          ],
        ),
      ),
      LibraryPreviewStatus.ready => Image.file(
        File(asset.previewPath),
        fit: BoxFit.cover,
        cacheWidth: (widget.width * MediaQuery.devicePixelRatioOf(context))
            .round()
            .clamp(64, 512),
        filterQuality: FilterQuality.low,
        errorBuilder: (context, error, stackTrace) {
          return const Center(child: Icon(Icons.broken_image_outlined));
        },
      ),
    };
  }

  void _openKeyboardMenu() {
    _menuController.open();
  }
}
