import "dart:async";
import "dart:math" as math;

import "package:material_ui/material_ui.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../../settings/domain/ame_preferences.dart";
import "../../application/library_source_read_scheduler.dart";
import "../../domain/library_models.dart";
import "../library_preview_images.dart";
import "../coordination/library_viewer_source_state.dart";
import "../coordination/library_viewer_zoom.dart";
import "library_viewer_controls.dart";
import "library_viewer_chrome.dart";
import "library_viewer_filmstrip_slot.dart";
import "library_viewer_frame.dart";
import "library_viewer_image.dart";
import "library_viewer_image_interaction.dart";
import "library_viewer_image_flight.dart";
import "library_path_text.dart";
import "library_viewer_source_notice.dart";
import "library_viewer_source_scope.dart";
import "library_viewer_toolbar.dart";
import "library_viewer_copy_scope.dart";

class LibraryImageViewer extends ConsumerStatefulWidget {
  const LibraryImageViewer({
    required this.asset,
    required this.onBack,
    required this.onInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    this.wheelBehavior = ImageViewerWheelBehavior.zoom,
    this.openBehavior = ImageViewerOpenBehavior.automatic,
    this.onPrevious,
    this.onNext,
    this.position,
    this.totalItems,
    this.filmstrip,
    this.sourceNotice,
    this.onReloadSource,
    this.organizationActions,
    this.operationNotice,
    super.key,
  });

  final LibraryAsset asset;
  final VoidCallback onBack;
  final VoidCallback onInformation;
  final VoidCallback onCopyPath;
  final VoidCallback onRevealFile;
  final ImageViewerWheelBehavior wheelBehavior;
  final ImageViewerOpenBehavior openBehavior;
  final VoidCallback? onPrevious;
  final VoidCallback? onNext;
  final int? position;
  final int? totalItems;
  final Widget? filmstrip;
  final LibraryViewerSourceNotice? sourceNotice;
  final VoidCallback? onReloadSource;
  final Widget? organizationActions;
  final Widget? operationNotice;

  @override
  ConsumerState<LibraryImageViewer> createState() => _LibraryImageViewerState();
}

class _LibraryImageViewerState extends ConsumerState<LibraryImageViewer>
    with SingleTickerProviderStateMixin {
  final _viewerFocusNode = FocusNode(debugLabel: "library-image-viewer");
  late final LibraryViewerZoom _zoom;

  @override
  void initState() {
    super.initState();
    _zoom = LibraryViewerZoom(vsync: this)
      ..addListener(_handlePresentationChanged);
    _requestViewerFocus();
  }

  void _handlePresentationChanged() {
    if (mounted) setState(() {});
  }

  @override
  void didUpdateWidget(covariant LibraryImageViewer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId) {
      _requestViewerFocus();
    }
  }

  @override
  void dispose() {
    _viewerFocusNode.dispose();
    _zoom.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LibraryViewerCopyScope(
      locationId: widget.asset.locationId,
      quarterTurns: _zoom.quarterTurns,
      child: LibraryViewerFrame(
        title: LibraryPathText(
          text: displayLibraryFileName(widget.asset.displayPath),
          path: widget.asset.displayPath,
          alwaysShowPathTooltip: true,
          textKey: const Key("viewer-source-path"),
        ),
        onBack: widget.onBack,
        actions: widget.organizationActions,
        builder: (context, isFullscreen, toggleFullscreen, goBack) =>
            CallbackShortcuts(
              bindings: {
                const SingleActivator(LogicalKeyboardKey.arrowLeft):
                    _showPreviousFromKeyboard,
                const SingleActivator(LogicalKeyboardKey.arrowRight):
                    _showNextFromKeyboard,
                ...libraryViewerZoomShortcuts(_zoom),
              },
              child: Focus(
                focusNode: _viewerFocusNode,
                autofocus: true,
                child: _buildCanvas(
                  context,
                  isFullscreen,
                  toggleFullscreen,
                  goBack,
                ),
              ),
            ),
      ),
    );
  }

  Widget _buildCanvas(
    BuildContext context,
    bool isFullscreen,
    VoidCallback? toggleFullscreen,
    VoidCallback goBack,
  ) {
    final copy = LibraryViewerCopyScope.of(context);
    final showFilmstrip =
        ref.watch(amePreferencesControllerProvider).viewerFilmstripVisible &&
        widget.filmstrip != null;
    return LibraryViewerBackdrop(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: LayoutBuilder(
        builder: (context, constraints) {
          _zoom.layout(
            image: widget.asset.locationId,
            imageSize: Size(
              widget.asset.width.toDouble(),
              widget.asset.height.toDouble(),
            ),
            viewport: Size(
              constraints.maxWidth,
              math.max(0, constraints.maxHeight - LibraryViewerToolbar.height),
            ),
            opening: widget.openBehavior,
          );
          return Column(
            children: [
              Expanded(
                child: Stack(
                  fit: StackFit.expand,
                  clipBehavior: Clip.none,
                  children: [
                    _buildImageViewport(goBack, copy),
                    if (widget.filmstrip case final filmstrip?)
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: LibraryViewerChrome(
                          child: LibraryViewerFilmstripSlot(
                            visible: showFilmstrip,
                            child: filmstrip,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
              LibraryViewerChrome(
                child: LibraryViewerToolbar(
                  asset: widget.asset,
                  positionLabel: _positionLabel,
                  onCopyPath: widget.onCopyPath,
                  onCopyImage: copy.copy,
                  onRevealFile: widget.onRevealFile,
                  isFilmstripVisible: showFilmstrip,
                  onToggleFilmstrip: widget.filmstrip == null
                      ? null
                      : _toggleFilmstrip,
                  onInformation: widget.onInformation,
                  onRotate: _zoom.hasDimensions ? _zoom.rotateClockwise : null,
                  isActualSize: _zoom.isActualSize,
                  onToggleZoomMode: _zoom.hasDimensions
                      ? _toggleActualSize
                      : null,
                  sliderValue: _zoom.sliderValue,
                  zoomPercent: _zoom.percent,
                  onSliderChanged: _zoom.sliderChanged,
                  sliderSemanticFormatter: _zoom.sliderLabel,
                  onZoomOut: _zoom.canZoomOut ? _zoomOut : null,
                  onZoomIn: _zoom.canZoomIn ? _zoomIn : null,
                  isFullscreen: isFullscreen,
                  onToggleFullscreen: toggleFullscreen,
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildImageViewport(VoidCallback goBack, LibraryViewerCopyData copy) {
    return Stack(
      fit: StackFit.expand,
      clipBehavior: Clip.none,
      children: [
        LibraryViewerImage(
          previewImages: ref.read(libraryPreviewImagesProvider),
          asset: widget.asset,
          sourceReadScheduler:
              LibraryViewerSourceScope.maybeOf(context)?.scheduler ??
              ref.watch(librarySourceReadSchedulerProvider),
          zoomScale: _zoom.transform.value.getMaxScaleOnAxis(),
          quarterTurns: _zoom.quarterTurns,
          imageBuilder: _buildTransformedImage,
          clipboard: copy.clipboard,
          operationNotice: copy.notice ?? widget.operationNotice,
          notice: switch (widget.sourceNotice) {
            null => null,
            final notice => LibraryViewerSourceNoticeBanner(
              notice: notice,
              onReload: widget.onReloadSource,
              onBack: goBack,
            ),
          },
        ),
        LibraryViewerNavigationButton.previous(
          onPressed: widget.onPrevious == null
              ? null
              : _showPreviousFromKeyboard,
        ),
        LibraryViewerNavigationButton.next(
          onPressed: widget.onNext == null ? null : _showNextFromKeyboard,
        ),
      ],
    );
  }

  Widget _buildTransformedImage(BuildContext context, Widget image) {
    return LibraryViewerImageInteraction(
      zoom: _zoom,
      wheelBehavior: widget.wheelBehavior,
      onPrevious: widget.onPrevious,
      onNext: widget.onNext,
      child: LibraryViewerImageFlight(
        locationId: widget.asset.locationId,
        imageSize: Size(
          widget.asset.width.toDouble(),
          widget.asset.height.toDouble(),
        ),
        quarterTurns: _zoom.quarterTurns,
        child: image,
      ),
    );
  }

  void _requestViewerFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _viewerFocusNode.requestFocus();
    });
  }

  void _toggleActualSize() {
    _zoom.toggleMode();
  }

  void _zoomIn() {
    _zoom.zoomIn();
  }

  void _zoomOut() {
    _zoom.zoomOut();
  }

  void _showPreviousFromKeyboard() {
    widget.onPrevious?.call();
  }

  void _showNextFromKeyboard() {
    widget.onNext?.call();
  }

  Future<void> _toggleFilmstrip() async {
    final preferences = ref.read(amePreferencesControllerProvider);
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(
            preferences.copyWith(
              viewerFilmstripVisible: !preferences.viewerFilmstripVisible,
            ),
          );
    } on Object {
      if (mounted) {
        showAmeTransientFeedback(
          context,
          message: "无法保存缩略图设置，请重试",
          severity: AmeNotificationSeverity.error,
        );
      }
    }
  }

  String? get _positionLabel {
    final position = widget.position;
    final totalItems = widget.totalItems;
    if (position == null || totalItems == null || totalItems <= 0) {
      return null;
    }
    return "$position / $totalItems 张";
  }
}
