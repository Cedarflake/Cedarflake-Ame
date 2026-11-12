import "dart:async";
import "dart:math" as math;
import "dart:ui" as ui;

import "package:flutter/painting.dart";

import "../../application/ports/library_image_clipboard.dart";

/// Borrows the viewer's existing image stream only for an explicit copy request.
class LibraryViewerClipboard {
  ImageProvider<Object>? _source;
  ImageStream? _stream;
  ImageStreamListener? _listener;
  Completer<ImageInfo?>? _frame;
  int _generation = 0;
  bool _copying = false;

  void bind(ImageProvider<Object> source) {
    if (_source == source) return;
    clear();
    _source = source;
  }

  void unbind(ImageProvider<Object> source) {
    if (_source == source) clear();
  }

  void clear() {
    _source = null;
    _generation += 1;
    final frame = _frame;
    _detach();
    if (frame != null && !frame.isCompleted) frame.complete(null);
  }

  Future<bool> copy(
    LibraryImageClipboard clipboard, {
    required int quarterTurns,
  }) async {
    final source = _source;
    if (_copying || source == null) return false;
    _copying = true;
    final generation = _generation;
    ImageInfo? frame;
    ui.Image? rotated;
    try {
      frame = await _readFrame(source);
      if (frame == null || generation != _generation) return false;
      var image = frame.image;
      if (image.width * image.height > LibraryImageClipboard.maximumPixels) {
        throw const ViewerClipboardImageTooLarge();
      }
      if (quarterTurns % 4 != 0) {
        rotated = await _rotate(image, quarterTurns % 4);
        image = rotated;
      }
      if (generation != _generation) return false;
      final png = await image.toByteData(format: ui.ImageByteFormat.png);
      if (generation != _generation) return false;
      final bytes = await image.toByteData(
        format: ui.ImageByteFormat.rawStraightRgba,
      );
      if (generation != _generation) return false;
      if (bytes == null || png == null) {
        throw StateError("The viewer frame could not be encoded");
      }
      await clipboard.write(
        width: image.width,
        height: image.height,
        rgba: bytes.buffer.asUint8List(
          bytes.offsetInBytes,
          bytes.lengthInBytes,
        ),
        png: png.buffer.asUint8List(png.offsetInBytes, png.lengthInBytes),
      );
      return generation == _generation;
    } on Object {
      if (generation != _generation) return false;
      rethrow;
    } finally {
      _detach();
      rotated?.dispose();
      frame?.dispose();
      _copying = false;
    }
  }

  Future<ui.Image> _rotate(ui.Image image, int turns) async {
    final width = turns.isOdd ? image.height : image.width;
    final height = turns.isOdd ? image.width : image.height;
    final recorder = ui.PictureRecorder();
    final canvas = ui.Canvas(recorder)
      ..translate(width / 2, height / 2)
      ..rotate(turns * math.pi / 2)
      ..translate(-image.width / 2, -image.height / 2);
    canvas.drawImage(image, ui.Offset.zero, ui.Paint());
    final picture = recorder.endRecording();
    try {
      return await picture.toImage(width, height);
    } finally {
      picture.dispose();
    }
  }

  Future<ImageInfo?> _readFrame(ImageProvider<Object> source) {
    final frame = Completer<ImageInfo?>();
    _frame = frame;
    _stream = source.resolve(ImageConfiguration.empty);
    _listener = ImageStreamListener(
      (image, synchronous) {
        if (frame.isCompleted) {
          image.dispose();
        } else {
          frame.complete(image);
        }
        _detach();
      },
      onError: (Object error, StackTrace? stack) {
        if (!frame.isCompleted) frame.completeError(error, stack);
        _detach();
      },
    );
    _stream!.addListener(_listener!);
    return frame.future;
  }

  void _detach() {
    final listener = _listener;
    if (listener != null) _stream?.removeListener(listener);
    _stream = null;
    _listener = null;
    _frame = null;
  }
}

class ViewerClipboardImageTooLarge implements Exception {
  const ViewerClipboardImageTooLarge();
}
