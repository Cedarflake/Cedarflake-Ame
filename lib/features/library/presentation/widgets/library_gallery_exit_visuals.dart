import "dart:math" as math;
import "dart:ui" as ui;

import "package:flutter/rendering.dart";
import "package:flutter/widgets.dart";

typedef LibraryGalleryPaintedItem = ({
  Object identity,
  GlobalKey boundary,
  Rect rect,
});

/// Retains pixels only: departing cards cannot keep reads, focus or actions alive.
class LibraryGalleryExitVisuals {
  static const _maximumItems = 64;
  static const _maximumPixels = 4 * 1024 * 1024;
  final _frames = <Object, _ExitFrame>{};

  bool get isEmpty => _frames.isEmpty;

  void capture(Iterable<LibraryGalleryPaintedItem> items, Rect viewport) {
    if (_frames.isNotEmpty) return;
    var remainingPixels = _maximumPixels;
    for (final item in items) {
      if (_frames.length == _maximumItems || remainingPixels <= 0) break;
      if (!item.rect.overlaps(viewport)) continue;
      final boundary = item.boundary.currentContext?.findRenderObject();
      if (boundary is! _GalleryPaintBoundary ||
          !boundary.attached ||
          !boundary.hasSize ||
          boundary.size.isEmpty) {
        continue;
      }
      final size = boundary.size;
      final ratio = math.min(1.0, 512 / math.max(size.width, size.height));
      final pixels = (size.width * ratio).ceil() * (size.height * ratio).ceil();
      if (pixels > remainingPixels) continue;
      final image = boundary.snapshotIfPainted(ratio);
      if (image == null) continue;
      _frames[item.identity] = _ExitFrame(image, item.rect);
      remainingPixels -= pixels;
    }
  }

  void removePresent(Set<Object> identities) {
    for (final identity in identities) {
      _frames.remove(identity)?.image.dispose();
    }
  }

  Widget build(Animation<double> opacity, Offset contentOffset) =>
      IgnorePointer(
        child: ExcludeSemantics(
          child: FadeTransition(
            opacity: opacity,
            child: Stack(
              children: [
                for (final entry in _frames.entries)
                  Positioned.fromRect(
                    key: ValueKey(("gallery-exit", entry.key)),
                    rect: entry.value.rect.shift(contentOffset),
                    child: RawImage(
                      image: entry.value.image,
                      fit: BoxFit.fill,
                      filterQuality: FilterQuality.low,
                    ),
                  ),
              ],
            ),
          ),
        ),
      );

  void clear() {
    for (final frame in _frames.values) {
      frame.image.dispose();
    }
    _frames.clear();
  }
}

class LibraryGalleryPaintBoundary extends SingleChildRenderObjectWidget {
  const LibraryGalleryPaintBoundary({required super.child, super.key});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _GalleryPaintBoundary();
}

class _GalleryPaintBoundary extends RenderRepaintBoundary {
  ui.Image? snapshotIfPainted(double ratio) {
    // A newly fading-in card can have layout without ever being painted.
    // Its missing layer must also be rejected in Release builds.
    if (layer == null) return null;
    var painted = true;
    assert(() {
      painted = !debugNeedsPaint;
      return true;
    }());
    return painted ? toImageSync(pixelRatio: ratio) : null;
  }
}

class _ExitFrame {
  const _ExitFrame(this.image, this.rect);
  final ui.Image image;
  final Rect rect;
}
