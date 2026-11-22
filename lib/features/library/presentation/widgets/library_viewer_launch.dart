import "package:material_ui/material_ui.dart";

typedef LibraryViewerGeometry = ({Rect bounds, Rect visibleBounds});

/// A single visible thumbnail, with no image lease or catalog authority.
class LibraryViewerOrigin {
  const LibraryViewerOrigin({
    required this.locationId,
    String? entryId,
    required this.geometry,
    required this.currentGeometry,
    required this.fit,
    required this.radius,
    this.findTarget,
    this.previewImage,
    this.previewQuality = FilterQuality.low,
  }) : entryId = entryId ?? locationId;

  final String locationId;
  final String entryId;
  final LibraryViewerGeometry geometry;
  final LibraryViewerGeometry? Function() currentGeometry;
  final BoxFit fit;
  final double radius;
  final LibraryViewerOrigin? Function(String)? findTarget;
  final ImageProvider<Object>? previewImage;
  final FilterQuality previewQuality;

  LibraryViewerOrigin? resolve() {
    final current = currentGeometry();
    return current == null
        ? null
        : LibraryViewerOrigin(
            locationId: locationId,
            entryId: entryId,
            geometry: current,
            currentGeometry: currentGeometry,
            fit: fit,
            radius: radius,
            findTarget: findTarget,
            previewImage: previewImage,
            previewQuality: previewQuality,
          );
  }
}

/// Keeps geometry suppliers only for currently mounted lazy thumbnails.
class LibraryViewerOrigins {
  final _sources =
      <
        Object,
        ({LibraryViewerOrigin? Function(String) read, bool returnTarget})
      >{};

  void register(
    Object owner,
    LibraryViewerOrigin? Function(String) read, {
    bool returnTarget = true,
  }) => _sources[owner] = (read: read, returnTarget: returnTarget);

  void unregister(Object owner) => _sources.remove(owner);

  LibraryViewerOrigin? find(String locationId) {
    for (final source in _sources.values) {
      if (!source.returnTarget) continue;
      final origin = source.read(locationId);
      if (origin != null) return origin;
    }
    return null;
  }

  LibraryViewerOrigin? findPreview(
    String entryId,
    ImageProvider<Object> image,
  ) {
    for (final entry in _sources.values) {
      final origin = entry.read(entryId);
      final preview = origin?.previewImage;
      final source = preview is ResizeImage ? preview.imageProvider : preview;
      if (source == image) return origin;
    }
    return null;
  }
}

class LibraryViewerOriginScope extends InheritedWidget {
  const LibraryViewerOriginScope({
    required this.prepare,
    required this.take,
    required this.origins,
    required super.child,
    super.key,
  });

  final ValueChanged<LibraryViewerOrigin?> prepare;
  final LibraryViewerOrigin? Function() take;
  final LibraryViewerOrigins origins;

  static LibraryViewerOriginScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryViewerOriginScope>();

  @override
  bool updateShouldNotify(LibraryViewerOriginScope oldWidget) =>
      origins != oldWidget.origins;
}

/// The registry retires each geometry supplier with its lazy thumbnail.
class LibraryViewerLaunch extends StatelessWidget {
  const LibraryViewerLaunch({
    required this.locationId,
    this.entryId,
    required this.onOpen,
    required this.builder,
    this.fit = BoxFit.cover,
    this.radius = 10,
    super.key,
  });

  final String locationId;
  final String? entryId;
  final VoidCallback onOpen;
  final BoxFit fit;
  final double radius;
  final Widget Function(BuildContext, Key, VoidCallback) builder;

  @override
  Widget build(BuildContext context) => LibraryViewerThumbnail(
    locationId: locationId,
    entryId: entryId,
    fit: fit,
    radius: radius,
    isReturnTarget: true,
    builder: (context, imageKey, readOrigin) => builder(context, imageKey, () {
      LibraryViewerOriginScope.of(context)?.prepare(readOrigin());
      onOpen();
    }),
  );
}

/// Registers only a mounted thumbnail; navigation does not change the return flight.
class LibraryViewerThumbnail extends StatefulWidget {
  const LibraryViewerThumbnail({
    required this.locationId,
    this.entryId,
    this.fit = BoxFit.cover,
    this.radius = 10,
    this.isReturnTarget = false,
    required this.builder,
    super.key,
  });

  final String locationId;
  final String? entryId;
  final BoxFit fit;
  final double radius;
  final bool isReturnTarget;
  final Widget Function(BuildContext, Key, LibraryViewerOrigin? Function())
  builder;

  @override
  State<LibraryViewerThumbnail> createState() => _LibraryViewerThumbnailState();
}

class _LibraryViewerThumbnailState extends State<LibraryViewerThumbnail> {
  final _thumbnailKey = GlobalKey();
  LibraryViewerOrigins? _origins;
  Object? _imageOwner;
  ImageProvider<Object>? _previewImage;
  FilterQuality _previewQuality = FilterQuality.low;

  void _publishImage(
    Object owner,
    ImageProvider<Object> image,
    FilterQuality quality,
  ) {
    _imageOwner = owner;
    _previewImage = image;
    _previewQuality = quality;
  }

  void _retireImage(Object owner) {
    if (!identical(owner, _imageOwner)) return;
    _imageOwner = null;
    _previewImage = null;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final origins = LibraryViewerOriginScope.of(context)?.origins;
    if (_origins == origins) return;
    _origins?.unregister(this);
    _origins = origins;
    origins?.register(this, _origin, returnTarget: widget.isReturnTarget);
  }

  @override
  void dispose() {
    _origins?.unregister(this);
    super.dispose();
  }

  LibraryViewerGeometry? _geometry(String entryId) {
    if (!mounted || (widget.entryId ?? widget.locationId) != entryId) {
      return null;
    }
    final box = _thumbnailKey.currentContext?.findRenderObject();
    if (box is! RenderBox ||
        !box.attached ||
        !box.hasSize ||
        box.size.isEmpty) {
      return null;
    }
    final bounds = MatrixUtils.transformRect(
      box.getTransformTo(null),
      Offset.zero & box.size,
    );
    if (!bounds.isFinite || bounds.isEmpty) return null;
    var visibleBounds = bounds;
    RenderObject descendant = box;
    for (
      var ancestor = box.parent;
      ancestor != null;
      ancestor = ancestor.parent
    ) {
      final clip = ancestor.describeApproximatePaintClip(descendant);
      if (clip != null) {
        visibleBounds = visibleBounds.intersect(
          MatrixUtils.transformRect(ancestor.getTransformTo(null), clip),
        );
        if (!visibleBounds.isFinite || visibleBounds.isEmpty) return null;
      }
      descendant = ancestor;
    }
    return (bounds: bounds, visibleBounds: visibleBounds);
  }

  LibraryViewerOrigin? _origin(String entryId) {
    final geometry = _geometry(entryId);
    return geometry == null
        ? null
        : LibraryViewerOrigin(
            locationId: widget.locationId,
            entryId: entryId,
            geometry: geometry,
            currentGeometry: () => _geometry(entryId),
            fit: widget.fit,
            radius: widget.radius,
            findTarget: _origins?.find,
            previewImage: _previewImage,
            previewQuality: _previewQuality,
          );
  }

  @override
  Widget build(BuildContext context) => LibraryViewerThumbnailScope(
    publish: _publishImage,
    retire: _retireImage,
    child: Builder(
      builder: (context) => widget.builder(
        context,
        _thumbnailKey,
        () => _origin(widget.entryId ?? widget.locationId),
      ),
    ),
  );
}

/// Shares the mounted thumbnail's decoded cache entry with its viewer.
class LibraryViewerThumbnailScope extends InheritedWidget {
  const LibraryViewerThumbnailScope({
    required this.publish,
    required this.retire,
    required super.child,
    super.key,
  });

  final void Function(Object, ImageProvider<Object>, FilterQuality) publish;
  final void Function(Object) retire;

  static LibraryViewerThumbnailScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryViewerThumbnailScope>();

  @override
  bool updateShouldNotify(LibraryViewerThumbnailScope oldWidget) =>
      publish != oldWidget.publish || retire != oldWidget.retire;
}

class LibraryViewerFlightScope extends InheritedWidget {
  const LibraryViewerFlightScope({
    required this.origin,
    required this.animation,
    required super.child,
    super.key,
  });

  final LibraryViewerOrigin? origin;
  final Animation<double> animation;

  static LibraryViewerFlightScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryViewerFlightScope>();

  @override
  bool updateShouldNotify(LibraryViewerFlightScope oldWidget) =>
      origin != oldWidget.origin || animation != oldWidget.animation;
}
