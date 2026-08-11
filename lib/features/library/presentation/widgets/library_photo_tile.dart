import "dart:async";
import "dart:io";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_controller.dart";
import "../../application/library_preview_store.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";

int libraryPreviewDecodeWidth(double logicalWidth, double devicePixelRatio) {
  final requestedWidth = (logicalWidth * devicePixelRatio).round().clamp(
    1,
    512,
  );
  if (requestedWidth <= 128) {
    return 128;
  }
  if (requestedWidth <= 256) {
    return 256;
  }
  return 512;
}

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
  late Stream<void> _previewChanges;
  LibraryPreviewSourceIdentity? _previewRepairSource;
  bool _isHovered = false;
  bool _isFocused = false;

  @override
  void initState() {
    super.initState();
    _controller = ref.read(libraryControllerProvider.notifier);
    _previewChanges = _controller.watchPreview(widget.asset.locationId);
  }

  @override
  void didUpdateWidget(covariant LibraryPhotoTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    final locationChanged =
        oldWidget.asset.locationId != widget.asset.locationId;
    if (locationChanged) {
      _previewChanges = _controller.watchPreview(widget.asset.locationId);
    }
    if (locationChanged ||
        !libraryPreviewSourcesAreCompatible(oldWidget.asset, widget.asset)) {
      _previewRepairSource = null;
    }
  }

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
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
            onPressed: () =>
                widget.onOpen(_controller.resolvePreview(widget.asset)),
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
                      onTap: () => widget.onOpen(
                        _controller.resolvePreview(widget.asset),
                      ),
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
    return StreamBuilder<void>(
      stream: _previewChanges,
      builder: (context, _) {
        return _buildPreviewAsset(
          context,
          _controller.resolvePreview(widget.asset),
        );
      },
    );
  }

  Widget _buildPreviewAsset(BuildContext context, LibraryAsset asset) {
    return switch (asset.previewStatus) {
      LibraryPreviewStatus.pending => const SizedBox.expand(
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
              onPressed: () => _controller.requestPreview(
                asset,
                retry: true,
                previewEdge: _requestedPreviewEdge(context),
              ),
              child: const Text(LibraryStrings.retryPreview),
            ),
          ],
        ),
      ),
      LibraryPreviewStatus.ready => _buildReadyPreview(context, asset),
    };
  }

  Widget _buildReadyPreview(BuildContext context, LibraryAsset asset) {
    final cacheWidth = libraryPreviewDecodeWidth(
      widget.width,
      MediaQuery.devicePixelRatioOf(context),
    );
    final previewEdge = _requestedPreviewEdge(context);
    return Image.file(
      File(asset.previewPath),
      fit: BoxFit.cover,
      cacheWidth: cacheWidth,
      gaplessPlayback: true,
      filterQuality: FilterQuality.low,
      errorBuilder: (context, error, stackTrace) {
        _schedulePreviewRepair(asset, cacheWidth, previewEdge);
        return Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.broken_image_outlined),
              const SizedBox(height: 4),
              TextButton(
                key: Key("preview-retry-${asset.locationId}"),
                onPressed: () {
                  _previewRepairSource = null;
                  _schedulePreviewRepair(asset, cacheWidth, previewEdge);
                },
                child: const Text(LibraryStrings.retryPreview),
              ),
            ],
          ),
        );
      },
    );
  }

  int _requestedPreviewEdge(BuildContext context) {
    final displayExtent = widget.width > widget.height
        ? widget.width
        : widget.height;
    return libraryPreviewDecodeWidth(
      displayExtent,
      MediaQuery.devicePixelRatioOf(context),
    );
  }

  void _schedulePreviewRepair(
    LibraryAsset asset,
    int cacheWidth,
    int previewEdge,
  ) {
    final source = LibraryPreviewSourceIdentity.fromAsset(asset);
    if (_previewRepairSource == source) {
      return;
    }
    _previewRepairSource = source;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      unawaited(_repairPreview(asset, cacheWidth, previewEdge));
    });
  }

  Future<void> _repairPreview(
    LibraryAsset asset,
    int cacheWidth,
    int previewEdge,
  ) async {
    if (asset.previewPath.isNotEmpty) {
      final provider = ResizeImage.resizeIfNeeded(
        cacheWidth,
        null,
        FileImage(File(asset.previewPath)),
      );
      try {
        await provider.evict();
      } on Object {
        // Cache eviction is best-effort; the backend still owns validation.
      }
    }
    if (mounted) {
      _controller.requestPreview(asset, retry: true, previewEdge: previewEdge);
    }
  }

  void _openKeyboardMenu() {
    _menuController.open();
  }
}
