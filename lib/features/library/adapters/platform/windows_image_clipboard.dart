import "package:flutter/services.dart";

import "../../application/ports/library_image_clipboard.dart";

class WindowsImageClipboard implements LibraryImageClipboard {
  const WindowsImageClipboard();

  static const _channel = MethodChannel("cedarflake_ame/image_clipboard");

  @override
  Future<void> write({
    required int width,
    required int height,
    required Uint8List rgba,
    required Uint8List png,
  }) => _channel.invokeMethod<void>("writeImage", {
    "width": width,
    "height": height,
    "rgba": rgba,
    "png": png,
  });
}
