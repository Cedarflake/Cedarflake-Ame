import "dart:async";
import "dart:math" as math;

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/window/ame_window_fullscreen.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../domain/library_models.dart";
import "../library_slideshow.dart";
import "library_viewer_controls.dart";
import "library_viewer_image.dart";
import "../library_viewer_zoom.dart";
import "library_viewer_toolbar.dart";
import "library_viewer_transition.dart";

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
    this.onSlideshowNext,
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
  final Future<bool> Function()? onSlideshowNext;

  @override
  ConsumerState<LibraryImageViewer> createState() => _LibraryImageViewerState();
}

class _LibraryImageViewerState extends ConsumerState<LibraryImageViewer>
    with SingleTickerProviderStateMixin {
  final _viewerFocusNode = FocusNode(debugLabel: "library-image-viewer");
  late final LibraryViewerZoom _zoom;
  DateTime? _lastWheelNavigationAt;
  final _slideshow = LibrarySlideshow();
  AmeWindowFullscreen? _fullscreen;

  @override
  void initState() {
    super.initState();
    _zoom = LibraryViewerZoom(vsync: this)
      ..addListener(_handlePresentationChanged);
    _slideshow.addListener(_handlePresentationChanged);
    _requestViewerFocus();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!LibraryViewerActivity.isActiveOf(context)) {
      _slideshow.pause();
      unawaited(_setFullscreen(false));
    }
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
    _slideshow.dispose();
    _fullscreen?.removeListener(_handlePresentationChanged);
    unawaited(_setFullscreen(false));
    _viewerFocusNode.dispose();
    _zoom.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final fullscreen = ref.watch(ameWindowFullscreenProvider);
    if (_fullscreen != fullscreen) {
      _fullscreen?.removeListener(_handlePresentationChanged);
      _fullscreen = fullscreen;
      _fullscreen?.addListener(_handlePresentationChanged);
    }
    _slideshow.show(widget.asset.locationId, widget.onSlideshowNext);
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): _escape,
        const SingleActivator(LogicalKeyboardKey.backspace): _back,
        const SingleActivator(LogicalKeyboardKey.browserBack): _back,
        const SingleActivator(LogicalKeyboardKey.f11): _toggleFullscreen,
        const SingleActivator(LogicalKeyboardKey.space): _slideshow.toggle,
        const SingleActivator(LogicalKeyboardKey.arrowLeft):
            _showPreviousFromKeyboard,
        const SingleActivator(LogicalKeyboardKey.arrowRight):
            _showNextFromKeyboard,
        const SingleActivator(LogicalKeyboardKey.add): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.equal, shift: true): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.minus): _zoomOut,
        const SingleActivator(LogicalKeyboardKey.numpadAdd): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.numpadSubtract): _zoomOut,
        const SingleActivator(LogicalKeyboardKey.equal, control: true): _zoomIn,
        const SingleActivator(
          LogicalKeyboardKey.equal,
          control: true,
          shift: true,
        ): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.minus, control: true):
            _zoomOut,
        const SingleActivator(LogicalKeyboardKey.digit0): _fitToWindow,
        const SingleActivator(LogicalKeyboardKey.digit0, control: true):
            _fitToWindow,
        const SingleActivator(LogicalKeyboardKey.digit1): _showActualSize,
        const SingleActivator(LogicalKeyboardKey.digit1, control: true):
            _showActualSize,
      },
      child: Focus(
        focusNode: _viewerFocusNode,
        autofocus: true,
        child: Column(
          children: [
            if (!(_fullscreen?.isFullscreen ?? false)) ...[
              LibraryViewerTopBar(
                displayPath: widget.asset.displayPath,
                onBack: _back,
              ),
              const Divider(height: 1),
            ],
            Expanded(child: _buildCanvas(context)),
          ],
        ),
      ),
    );
  }

  Widget _buildCanvas(BuildContext context) {
    final showFilmstrip =
        ref.watch(amePreferencesControllerProvider).viewerFilmstripVisible &&
        widget.filmstrip != null;
    return ColoredBox(
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
              math.max(
                0,
                constraints.maxHeight -
                    LibraryViewerToolbar.height -
                    (showFilmstrip ? 104 : 0),
              ),
            ),
            opening: widget.openBehavior,
          );
          return Column(
            children: [
              Expanded(child: _buildImageViewport()),
              if (showFilmstrip)
                Listener(
                  onPointerDown: (_) => _slideshow.pause(),
                  onPointerSignal: (_) => _slideshow.pause(),
                  child: SizedBox(height: 104, child: widget.filmstrip),
                ),
              LibraryViewerToolbar(
                asset: widget.asset,
                positionLabel: _positionLabel,
                onMenuOpened: _slideshow.pause,
                onCopyPath: widget.onCopyPath,
                onRevealFile: widget.onRevealFile,
                isFilmstripVisible: showFilmstrip,
                onToggleFilmstrip: widget.filmstrip == null
                    ? null
                    : _toggleFilmstrip,
                isPlaying: _slideshow.isPlaying,
                onToggleSlideshow: _slideshow.isPlaying || _slideshow.canPlay
                    ? _slideshow.toggle
                    : null,
                onInformation: () {
                  _slideshow.pause();
                  widget.onInformation();
                },
                isActualSize: _zoom.isActualSize,
                onToggleZoomMode: _zoom.hasDimensions
                    ? _toggleActualSize
                    : null,
                sliderValue: _zoom.sliderValue,
                zoomPercent: _zoom.percent,
                onSliderChanged: (value) {
                  _slideshow.pause();
                  _zoom.sliderChanged(value);
                },
                sliderSemanticFormatter: _zoom.sliderLabel,
                onZoomOut: _zoom.canZoomOut ? _zoomOut : null,
                onZoomIn: _zoom.canZoomIn ? _zoomIn : null,
                isFullscreen: _fullscreen?.isFullscreen ?? false,
                onToggleFullscreen:
                    _fullscreen == null || _fullscreen!.isChanging
                    ? null
                    : _toggleFullscreen,
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildImageViewport() {
    return Stack(
      fit: StackFit.expand,
      children: [
        LibraryViewerImage(
          asset: widget.asset,
          imageBuilder: _buildTransformedImage,
          onReady: () => _slideshow.ready(widget.asset.locationId),
          onFailed: () => _slideshow.failed(widget.asset.locationId),
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
    return Listener(
      onPointerSignal: _handlePointerSignal,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onDoubleTap: _toggleActualSize,
        child: InteractiveViewer(
          key: const Key("library-image-interactive-viewer"),
          transformationController: _zoom.transform,
          alignment: Alignment.topLeft,
          boundaryMargin: const EdgeInsets.all(double.infinity),
          minScale: _zoom.minimumScale,
          maxScale: _zoom.maximumScale,
          scaleFactor: 180,
          scaleEnabled: widget.wheelBehavior == ImageViewerWheelBehavior.zoom,
          trackpadScrollCausesScale: true,
          onInteractionStart: _handleInteractionStart,
          child: SizedBox(
            width: _zoom.viewport.width,
            height: _zoom.viewport.height,
            child: image,
          ),
        ),
      ),
    );
  }

  void _requestViewerFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _viewerFocusNode.requestFocus();
    });
  }

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || event.scrollDelta.dy == 0) {
      return;
    }
    if (widget.wheelBehavior == ImageViewerWheelBehavior.zoom) {
      _zoom.animatePointerZoom();
      return;
    }
    final now = DateTime.now();
    final lastNavigation = _lastWheelNavigationAt;
    if (lastNavigation != null &&
        now.difference(lastNavigation) < const Duration(milliseconds: 240)) {
      return;
    }
    _lastWheelNavigationAt = now;
    if (event.scrollDelta.dy < 0) {
      _showPreviousFromKeyboard();
    } else {
      _showNextFromKeyboard();
    }
  }

  void _handleInteractionStart(ScaleStartDetails details) {
    _slideshow.pause();
    _zoom.interactionStarted(details);
  }

  void _fitToWindow() {
    _slideshow.pause();
    _zoom.fit();
  }

  void _showActualSize() {
    _slideshow.pause();
    _zoom.actualSize();
  }

  void _toggleActualSize() {
    _slideshow.pause();
    _zoom.toggleMode();
  }

  void _zoomIn() {
    _slideshow.pause();
    _zoom.zoomIn();
  }

  void _zoomOut() {
    _slideshow.pause();
    _zoom.zoomOut();
  }

  void _showPreviousFromKeyboard() {
    _slideshow.pause();
    widget.onPrevious?.call();
  }

  void _showNextFromKeyboard() {
    _slideshow.pause();
    widget.onNext?.call();
  }

  void _escape() {
    if (_fullscreen?.isFullscreen ?? false) {
      unawaited(_setFullscreen(false));
    } else if (_slideshow.isPlaying) {
      _slideshow.pause();
    } else {
      _back();
    }
  }

  Future<void> _back() async {
    _slideshow.pause();
    await _setFullscreen(false);
    if (mounted) widget.onBack();
  }

  void _toggleFullscreen() =>
      unawaited(_setFullscreen(!(_fullscreen?.isFullscreen ?? false)));

  Future<void> _setFullscreen(bool enabled) async {
    try {
      await _fullscreen?.setEnabled(enabled);
    } on Object {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("无法切换全屏，请重试")));
      }
    }
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
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("无法保存缩略图设置，请重试")));
      }
    }
  }

  String? get _positionLabel {
    final position = widget.position;
    final totalItems = widget.totalItems;
    if (position == null || totalItems == null || totalItems <= 0) {
      return null;
    }
    return "$position / $totalItems";
  }
}
