import "package:material_ui/material_ui.dart";

typedef LibraryViewerGeometry = ({Rect bounds, Rect visibleBounds});

/// A single visible thumbnail, with no image lease or catalog authority.
class LibraryViewerOrigin {
  const LibraryViewerOrigin({
    required this.locationId,
    required this.geometry,
    required this.currentGeometry,
    required this.fit,
    required this.radius,
    this.findTarget,
  });

  final String locationId;
  final LibraryViewerGeometry geometry;
  final LibraryViewerGeometry? Function() currentGeometry;
  final BoxFit fit;
  final double radius;
  final LibraryViewerOrigin? Function(String)? findTarget;

  LibraryViewerOrigin? resolve() {
    final current = currentGeometry();
    return current == null
        ? null
        : LibraryViewerOrigin(
            locationId: locationId,
            geometry: current,
            currentGeometry: currentGeometry,
            fit: fit,
            radius: radius,
            findTarget: findTarget,
          );
  }
}

/// Keeps geometry suppliers only for currently mounted lazy thumbnails.
class LibraryViewerOrigins {
  final _sources = <Object, LibraryViewerOrigin? Function(String)>{};

  void register(Object owner, LibraryViewerOrigin? Function(String) read) =>
      _sources[owner] = read;

  void unregister(Object owner) => _sources.remove(owner);

  LibraryViewerOrigin? find(String locationId) {
    for (final read in _sources.values) {
      final origin = read(locationId);
      if (origin != null) return origin;
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
class LibraryViewerLaunch extends StatefulWidget {
  const LibraryViewerLaunch({
    required this.locationId,
    required this.onOpen,
    required this.builder,
    this.fit = BoxFit.cover,
    this.radius = 10,
    super.key,
  });

  final String locationId;
  final VoidCallback onOpen;
  final BoxFit fit;
  final double radius;
  final Widget Function(BuildContext, Key, VoidCallback) builder;

  @override
  State<LibraryViewerLaunch> createState() => _LibraryViewerLaunchState();
}

class _LibraryViewerLaunchState extends State<LibraryViewerLaunch> {
  final _thumbnailKey = GlobalKey();
  LibraryViewerOrigins? _origins;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final origins = LibraryViewerOriginScope.of(context)?.origins;
    if (_origins == origins) return;
    _origins?.unregister(this);
    _origins = origins;
    origins?.register(this, _origin);
  }

  @override
  void dispose() {
    _origins?.unregister(this);
    super.dispose();
  }

  LibraryViewerGeometry? _geometry(String locationId) {
    if (!mounted || widget.locationId != locationId) return null;
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

  LibraryViewerOrigin? _origin(String locationId) {
    final geometry = _geometry(locationId);
    return geometry == null
        ? null
        : LibraryViewerOrigin(
            locationId: locationId,
            geometry: geometry,
            currentGeometry: () => _geometry(locationId),
            fit: widget.fit,
            radius: widget.radius,
            findTarget: _origins?.find,
          );
  }

  void _open() {
    LibraryViewerOriginScope.of(context)?.prepare(_origin(widget.locationId));
    widget.onOpen();
  }

  @override
  Widget build(BuildContext context) =>
      widget.builder(context, _thumbnailKey, _open);
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
