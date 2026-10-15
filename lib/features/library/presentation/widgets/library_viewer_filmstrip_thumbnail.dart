import "dart:async";
import "dart:io";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_controller.dart";
import "../../application/library_preview_store.dart";
import "../../domain/library_models.dart";
import "library_image_reveal.dart";
import "library_path_text.dart";
import "library_photo_tile.dart" show libraryPreviewDecodeWidth;
import "library_viewer_transition.dart";

class LibraryViewerFilmstripThumbnail extends ConsumerStatefulWidget {
  const LibraryViewerFilmstripThumbnail({
    required this.asset,
    required this.isSelected,
    required this.onPressed,
    super.key,
  });

  final LibraryAsset asset;
  final bool isSelected;
  final VoidCallback onPressed;

  @override
  ConsumerState<LibraryViewerFilmstripThumbnail> createState() =>
      _LibraryViewerFilmstripThumbnailState();
}

class _LibraryViewerFilmstripThumbnailState
    extends ConsumerState<LibraryViewerFilmstripThumbnail> {
  late final _controller = ref.read(libraryControllerProvider.notifier);
  LibraryPreviewSourceIdentity? _repairSource;
  late StreamSubscription<void> _previewChanges;
  ImageProvider<Object>? _repairImage;
  Object? _repair;
  bool _awaitingPublication = false;
  int _imageGeneration = 0;
  bool _isActive = true;

  @override
  void initState() {
    super.initState();
    _previewChanges = _controller
        .watchPreview(widget.asset.locationId)
        .listen((_) => _previewChanged());
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _isActive =
        LibraryViewerActivity.isActiveOf(context) && Visibility.of(context);
    if (!_isActive && _repair != null) {
      _repairSource = null;
      _repair = null;
      _repairImage = null;
      _awaitingPublication = false;
    }
  }

  @override
  void didUpdateWidget(covariant LibraryViewerFilmstripThumbnail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!libraryPreviewSourcesAreCompatible(oldWidget.asset, widget.asset)) {
      _repairSource = null;
      _repair = null;
      _repairImage = null;
      _awaitingPublication = false;
    }
    if (oldWidget.asset.locationId != widget.asset.locationId) {
      unawaited(_previewChanges.cancel());
      _previewChanges = _controller
          .watchPreview(widget.asset.locationId)
          .listen((_) => _previewChanged());
    }
  }

  @override
  void dispose() {
    unawaited(_previewChanges.cancel());
    super.dispose();
  }

  void _previewChanged() {
    if (!mounted) return;
    final current = _controller.resolvePreview(widget.asset);
    final repair = _repair;
    final source = _repairSource;
    final image = _repairImage;
    if (_awaitingPublication &&
        repair != null &&
        source != null &&
        image != null) {
      _awaitingPublication = false;
      if (current.previewStatus == LibraryPreviewStatus.ready &&
          _ownsRepair(repair, source)) {
        unawaited(_refreshRepairedImage(image, repair, source));
      } else {
        _repair = null;
        _repairImage = null;
      }
    }
    if (_isActive) setState(() {});
  }

  bool _ownsRepair(Object repair, LibraryPreviewSourceIdentity source) =>
      mounted &&
      _isActive &&
      identical(_repair, repair) &&
      source.isCompatibleWith(_controller.resolvePreview(widget.asset));

  void _scheduleRepair(
    LibraryAsset asset,
    ImageProvider<Object> image,
    int edge,
  ) {
    final source = LibraryPreviewSourceIdentity.fromAsset(asset);
    if (!_isActive || _repairSource == source) return;
    _repairSource = source;
    final repair = _repair = Object();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_ownsRepair(repair, source)) return;
      final current = _controller.resolvePreview(widget.asset);
      if (current.previewStatus != LibraryPreviewStatus.ready ||
          current.previewPath != asset.previewPath) {
        _repair = null;
        return;
      }
      unawaited(_requestRepair(asset, image, edge, repair, source));
    });
  }

  Future<void> _requestRepair(
    LibraryAsset asset,
    ImageProvider<Object> image,
    int edge,
    Object repair,
    LibraryPreviewSourceIdentity source,
  ) async {
    try {
      await image.evict();
      if (!_ownsRepair(repair, source)) return;
      final current = _controller.resolvePreview(widget.asset);
      if (current.previewStatus != LibraryPreviewStatus.ready ||
          current.previewPath != asset.previewPath) {
        _repair = null;
        return;
      }
      _repairImage = image;
      _awaitingPublication = true;
      // Automatic repair follows visible demand, so hiding can retire queued work.
      _controller.requestPreview(asset, retry: true, previewEdge: edge);
    } on Object catch (error, stack) {
      if (identical(_repair, repair)) {
        _repair = null;
        _repairImage = null;
        _awaitingPublication = false;
      }
      FlutterError.reportError(
        FlutterErrorDetails(exception: error, stack: stack),
      );
    }
  }

  Future<void> _refreshRepairedImage(
    ImageProvider<Object> image,
    Object repair,
    LibraryPreviewSourceIdentity source,
  ) async {
    try {
      await image.evict();
      if (_ownsRepair(repair, source)) setState(() => _imageGeneration++);
    } finally {
      if (identical(_repair, repair)) {
        _repair = null;
        _repairImage = null;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 3),
      child: Semantics(
        button: true,
        selected: widget.isSelected,
        label: displayLibraryFileName(widget.asset.displayPath),
        child: Material(
          clipBehavior: Clip.antiAlias,
          color: colors.surfaceContainerHigh,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(6),
            side: BorderSide(
              color: widget.isSelected ? colors.primary : Colors.transparent,
              width: 2,
            ),
          ),
          child: InkWell(
            onTap: widget.onPressed,
            child: Padding(
              padding: const EdgeInsets.all(3),
              child: ExcludeSemantics(
                child: Builder(
                  builder: (context) {
                    final resolved = _controller.resolvePreview(widget.asset);
                    if (resolved.previewStatus != LibraryPreviewStatus.ready) {
                      return Icon(
                        resolved.previewStatus == LibraryPreviewStatus.failed
                            ? Symbols.broken_image_rounded
                            : Symbols.image_rounded,
                        size: 22,
                      );
                    }
                    final source = LibraryPreviewSourceIdentity.fromAsset(
                      resolved,
                    );
                    final edge = libraryPreviewDecodeWidth(
                      94,
                      MediaQuery.devicePixelRatioOf(context),
                    );
                    final image = ResizeImage.resizeIfNeeded(
                      edge,
                      null,
                      FileImage(File(resolved.previewPath)),
                    );
                    return LibraryImageReveal(
                      key: ValueKey((source, _imageGeneration)),
                      image: image,
                      revealIdentity: source,
                      fit: BoxFit.contain,
                      placeholder: const Icon(Symbols.image_rounded, size: 22),
                      errorBuilder: (context, error, stack) {
                        _scheduleRepair(resolved, image, edge);
                        return const Icon(
                          Symbols.broken_image_rounded,
                          size: 22,
                        );
                      },
                    );
                  },
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
