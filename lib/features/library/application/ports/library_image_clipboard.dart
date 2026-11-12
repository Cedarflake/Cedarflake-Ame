import "dart:typed_data";

abstract interface class LibraryImageClipboard {
  static const maximumPixels = 32 * 1024 * 1024;

  /// Publishes equivalent PNG and straight-alpha RGBA representations of a frame.
  Future<void> write({
    required int width,
    required int height,
    required Uint8List rgba,
    required Uint8List png,
  });
}
