import "dart:math" as math;

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../application/library_controller.dart";
import "../../application/library_source_read_scheduler.dart";
import "../../application/library_source_reader.dart";
import "../../domain/library_models.dart";
import "../coordination/library_comparison_view.dart";
import "../coordination/library_viewer_zoom.dart";
import "../library_duplicate_strings.dart";
import "../library_preview_images.dart";
import "library_path_text.dart";
import "library_viewer_frame.dart";
import "library_viewer_image.dart";
import "library_viewer_image_interaction.dart";
import "library_viewer_toolbar.dart";
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

class _LibraryImageComparisonState extends ConsumerState<LibraryImageComparison>
    with TickerProviderStateMixin {
  static const _paneInset = 2.0;
  static const _headingHeight = 48.0;
  late final _view = LibraryComparisonView(vsync: this);
  final _previewOwner = Object();
  late final _library = ref.read(libraryControllerProvider.notifier);
  late final _leftReads = LibrarySourceReadScheduler(
    reader: ref.read(librarySourceReaderProvider),
  );
  late final _rightReads = LibrarySourceReadScheduler(
    reader: ref.read(librarySourceReaderProvider),
  );
  bool _closing = false;

  void _back() {
    setState(() => _closing = true);
    _library.releaseFilmstripPreviewDemand(_previewOwner);
    widget.onBack();
  }

  @override
  void initState() {
    super.initState();
    _library.updateFilmstripPreviewDemand(_previewOwner, [
      widget.left,
      widget.right,
    ], 512);
  }

  @override
  void dispose() {
    _library.releaseFilmstripPreviewDemand(_previewOwner);
    _leftReads.dispose();
    _rightReads.dispose();
    _view.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LibraryViewerActivity(
    isActive: !_closing,
    child: Scaffold(
      body: ListenableBuilder(
        listenable: _view,
        builder: (context, child) => LibraryViewerFrame(
          title: const Text(
            LibraryDuplicateStrings.compare,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          backLabel: LibraryDuplicateStrings.back,
          onBack: _back,
          builder: (context, isFullscreen, toggleFullscreen, _) =>
              LayoutBuilder(
                builder: (context, constraints) {
                  final height = math.max(
                    0.0,
                    constraints.maxHeight - LibraryViewerToolbar.height,
                  );
                  final viewport = Size(
                    math.max(
                      0.0,
                      (constraints.maxWidth - 1) / 2 - _paneInset * 2,
                    ),
                    math.max(0.0, height - _headingHeight - _paneInset * 2),
                  );
                  final opening = ref
                      .watch(amePreferencesControllerProvider)
                      .viewerOpenBehavior;
                  for (final pane in [
                    (widget.left, _view.left),
                    (widget.right, _view.right),
                  ]) {
                    pane.$2.layout(
                      image: pane.$1.locationId,
                      imageSize: Size(
                        pane.$1.width.toDouble(),
                        pane.$1.height.toDouble(),
                      ),
                      viewport: viewport,
                      opening: opening,
                    );
                  }
                  return CallbackShortcuts(
                    bindings: {
                      ...libraryViewerZoomShortcuts(_view.activeZoom),
                      const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
                          _view.select(LibraryComparisonSide.left),
                      const SingleActivator(
                        LogicalKeyboardKey.arrowRight,
                      ): () =>
                          _view.select(LibraryComparisonSide.right),
                    },
                    child: Focus(
                      autofocus: true,
                      child: Column(
                        children: [
                          Expanded(
                            child: Row(
                              children: [
                                Expanded(
                                  child: _pane(
                                    widget.left,
                                    _view.left,
                                    _leftReads,
                                    LibraryComparisonSide.left,
                                  ),
                                ),
                                const VerticalDivider(width: 1),
                                Expanded(
                                  child: _pane(
                                    widget.right,
                                    _view.right,
                                    _rightReads,
                                    LibraryComparisonSide.right,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          LibraryViewerToolbar(
                            asset: _view.active == LibraryComparisonSide.left
                                ? widget.left
                                : widget.right,
                            leading: _comparisonActions(),
                            isActualSize: _view.activeZoom.isActualSize,
                            onToggleZoomMode: _view.activeZoom.toggleMode,
                            sliderValue: _view.activeZoom.sliderValue,
                            zoomPercent: _view.activeZoom.percent,
                            onSliderChanged: _view.activeZoom.sliderChanged,
                            sliderSemanticFormatter:
                                _view.activeZoom.sliderLabel,
                            onZoomOut: _view.activeZoom.canZoomOut
                                ? _view.activeZoom.zoomOut
                                : null,
                            onZoomIn: _view.activeZoom.canZoomIn
                                ? _view.activeZoom.zoomIn
                                : null,
                            isFullscreen: isFullscreen,
                            onToggleFullscreen: toggleFullscreen,
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),
        ),
      ),
    ),
  );

  Widget _comparisonActions() => Row(
    children: [
      const Spacer(),
      LibraryViewerToolbarButton(
        id: "comparison-link",
        label: LibraryDuplicateStrings.syncComparison,
        selected: _view.linked,
        onPressed: _view.toggleLinked,
        icon: _view.linked ? Symbols.link_rounded : Symbols.link_off_rounded,
      ),
      LibraryViewerToolbarButton(
        id: "comparison-reset",
        label: LibraryDuplicateStrings.resetComparison,
        onPressed: _view.fitBoth,
        icon: Symbols.fit_screen_rounded,
      ),
    ],
  );

  Widget _pane(
    LibraryAsset asset,
    LibraryViewerZoom zoom,
    LibrarySourceReadScheduler sourceReads,
    LibraryComparisonSide side,
  ) => Listener(
    onPointerDown: (_) => _view.select(side),
    onPointerSignal: (event) {
      if (event is PointerScrollEvent) _view.select(side);
    },
    child: DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(
          color: _view.active == side
              ? Theme.of(context).colorScheme.primary
              : Colors.transparent,
          width: _paneInset,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(_paneInset),
        child: Column(
          children: [
            SizedBox(
              height: _headingHeight,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Center(
                  child: LibraryPathText(
                    text: displayLibraryFileName(asset.displayPath),
                    path: asset.displayPath,
                  ),
                ),
              ),
            ),
            Expanded(
              child: StreamBuilder<void>(
                stream: _library.watchPreview(asset.locationId),
                builder: (context, snapshot) => LibraryViewerImage(
                  asset: _library.resolvePreview(asset),
                  previewImages: ref.read(libraryPreviewImagesProvider),
                  sourceReadScheduler: sourceReads,
                  zoomScale: zoom.transform.value.getMaxScaleOnAxis(),
                  imageBuilder: (context, image) =>
                      LibraryViewerImageInteraction(
                        zoom: zoom,
                        interactiveKey: ValueKey(
                          "comparison-image-${side.name}",
                        ),
                        child: image,
                      ),
                ),
              ),
            ),
          ],
        ),
      ),
    ),
  );
}
