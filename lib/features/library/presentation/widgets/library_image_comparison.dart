import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_controller.dart";
import "../../application/library_source_read_scheduler.dart";
import "../../application/library_source_reader.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "../library_preview_images.dart";
import "library_path_text.dart";
import "library_viewer_image.dart";
import "library_viewer_transition.dart";

class LibraryImageComparison extends ConsumerStatefulWidget {
  const LibraryImageComparison({
    required this.left,
    required this.right,
    required this.onBack,
    super.key,
  });
  final LibraryAsset left;
  final LibraryAsset right;
  final VoidCallback onBack;

  @override
  ConsumerState<LibraryImageComparison> createState() =>
      _LibraryImageComparisonState();
}

class _LibraryImageComparisonState
    extends ConsumerState<LibraryImageComparison> {
  final _left = TransformationController();
  final _right = TransformationController();
  final _previewOwner = Object();
  late final _library = ref.read(libraryControllerProvider.notifier);
  late final _leftReads = LibrarySourceReadScheduler(
    reader: ref.read(librarySourceReaderProvider),
  );
  late final _rightReads = LibrarySourceReadScheduler(
    reader: ref.read(librarySourceReaderProvider),
  );
  bool _linked = true;
  bool _synchronizing = false;
  bool _closing = false;

  void _back() {
    setState(() => _closing = true);
    _library.releaseFilmstripPreviewDemand(_previewOwner);
    widget.onBack();
  }

  @override
  void initState() {
    super.initState();
    _left.addListener(_leftChanged);
    _right.addListener(_rightChanged);
    _library.updateFilmstripPreviewDemand(_previewOwner, [
      widget.left,
      widget.right,
    ], 512);
  }

  void _leftChanged() => _synchronize(_left, _right);
  void _rightChanged() => _synchronize(_right, _left);
  void _synchronize(
    TransformationController source,
    TransformationController target,
  ) {
    if (_synchronizing || !_linked) return;
    _synchronizing = true;
    target.value = source.value.clone();
    _synchronizing = false;
  }

  void _reset() {
    _left.value = Matrix4.identity();
    _right.value = Matrix4.identity();
  }

  void _scale(TransformationController controller, double factor) {
    final scale = controller.value.getMaxScaleOnAxis();
    final target = (scale * factor).clamp(1.0, 32.0);
    controller.value = controller.value.clone()
      ..scaleByDouble(target / scale, target / scale, 1, 1);
  }

  @override
  void dispose() {
    _library.releaseFilmstripPreviewDemand(_previewOwner);
    _leftReads.dispose();
    _rightReads.dispose();
    _left.dispose();
    _right.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LibraryViewerActivity(
    isActive: !_closing,
    child: CallbackShortcuts(
      bindings: {const SingleActivator(LogicalKeyboardKey.escape): _back},
      child: Focus(
        autofocus: true,
        child: Scaffold(
          appBar: AppBar(
            automaticallyImplyLeading: false,
            leading: IconButton(
              tooltip: LibraryDuplicateStrings.back,
              onPressed: _back,
              icon: const Icon(Symbols.arrow_back_rounded),
            ),
            title: const Text(LibraryDuplicateStrings.compare),
            actions: [
              IconButton(
                tooltip: LibraryDuplicateStrings.syncComparison,
                isSelected: _linked,
                onPressed: () {
                  setState(() => _linked = !_linked);
                  if (_linked) _synchronize(_left, _right);
                },
                icon: const Icon(Symbols.link_off_rounded),
                selectedIcon: const Icon(Symbols.link_rounded),
              ),
              IconButton(
                tooltip: LibraryDuplicateStrings.resetComparison,
                onPressed: _reset,
                icon: const Icon(Symbols.fit_screen_rounded),
              ),
            ],
          ),
          body: LayoutBuilder(
            builder: (context, constraints) {
              final vertical = constraints.maxWidth < 600;
              return Flex(
                direction: vertical ? Axis.vertical : Axis.horizontal,
                children: [
                  Expanded(
                    child: _pane(widget.left, _left, _leftReads, "left"),
                  ),
                  if (vertical)
                    const Divider(height: 1)
                  else
                    const VerticalDivider(width: 1),
                  Expanded(
                    child: _pane(widget.right, _right, _rightReads, "right"),
                  ),
                ],
              );
            },
          ),
        ),
      ),
    ),
  );

  Widget _pane(
    LibraryAsset asset,
    TransformationController transform,
    LibrarySourceReadScheduler sourceReads,
    String side,
  ) => Column(
    children: [
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: Row(
          children: [
            Expanded(
              child: LibraryPathText(
                text: displayLibraryFileName(asset.displayPath),
                path: asset.displayPath,
              ),
            ),
            AmeTooltip(
              message: LibraryDuplicateStrings.zoomOut,
              child: IconButton(
                onPressed: () => _scale(transform, 1 / 1.25),
                icon: const Icon(Symbols.remove_rounded),
              ),
            ),
            AmeTooltip(
              message: LibraryDuplicateStrings.zoomIn,
              child: IconButton(
                key: ValueKey("comparison-zoom-$side"),
                onPressed: () => _scale(transform, 1.25),
                icon: const Icon(Symbols.add_rounded),
              ),
            ),
          ],
        ),
      ),
      Expanded(
        child: StreamBuilder<void>(
          stream: _library.watchPreview(asset.locationId),
          builder: (context, snapshot) => AnimatedBuilder(
            animation: transform,
            builder: (context, child) => LibraryViewerImage(
              asset: _library.resolvePreview(asset),
              previewImages: ref.read(libraryPreviewImagesProvider),
              sourceReadScheduler: sourceReads,
              zoomScale: transform.value.getMaxScaleOnAxis(),
              imageBuilder: (context, image) => InteractiveViewer(
                key: ValueKey("comparison-image-$side"),
                transformationController: transform,
                minScale: 1,
                maxScale: 32,
                child: SizedBox.expand(child: image),
              ),
            ),
          ),
        ),
      ),
    ],
  );
}
