import "dart:ui" as ui;

import "package:flutter/foundation.dart";
import "package:flutter/painting.dart";

import "../../application/library_source_reader.dart";

ImageStreamCompleter createLibrarySourceImageStream({
  required Future<ui.Codec> codec,
  required bool Function() isRetired,
  required String debugLabel,
}) => _SourceImageStreamCompleter(
  codec: codec,
  isRetired: isRetired,
  debugLabel: debugLabel,
);

class _SourceImageStreamCompleter extends MultiFrameImageStreamCompleter {
  _SourceImageStreamCompleter({
    required Future<ui.Codec> codec,
    required bool Function() isRetired,
    required String debugLabel,
  }) : _isRetired = isRetired,
       super(
         codec: _bindCodec(codec, isRetired),
         scale: 1,
         debugLabel: debugLabel,
       );

  final bool Function() _isRetired;

  @override
  void reportError({
    DiagnosticsNode? context,
    required Object exception,
    StackTrace? stack,
    InformationCollector? informationCollector,
    bool silent = false,
  }) {
    // Native codec and frame futures may complete after Flutter has disposed
    // the stream. Only the actual retired viewer intent loses error authority.
    if (_isRetired()) {
      return;
    }
    super.reportError(
      context: context,
      exception: exception,
      stack: stack,
      informationCollector: informationCollector,
      silent: silent,
    );
  }
}

Future<ui.Codec> _bindCodec(
  Future<ui.Codec> result,
  bool Function() isRetired,
) => result.then((codec) {
  if (isRetired()) {
    codec.dispose();
    throw _retiredSource();
  }
  return _SourceImageCodec(codec, isRetired);
});

class _SourceImageCodec implements ui.Codec {
  _SourceImageCodec(this._inner, this._isRetired);

  final ui.Codec _inner;
  final bool Function() _isRetired;
  bool _disposed = false;

  @override
  int get frameCount => _inner.frameCount;

  @override
  int get repetitionCount => _inner.repetitionCount;

  @override
  Future<ui.FrameInfo> getNextFrame() async {
    if (_disposed || _isRetired()) {
      throw _retiredSource();
    }
    final frame = await _inner.getNextFrame();
    // The SDK returns from its late-frame path after noticing a disposed codec
    // without releasing that newly delivered image. It has not received ours.
    if (_disposed || _isRetired()) {
      frame.image.dispose();
      throw _retiredSource();
    }
    return frame;
  }

  @override
  void dispose() {
    if (!_disposed) {
      _disposed = true;
      _inner.dispose();
    }
  }
}

LibrarySourceReadFailure _retiredSource() => const LibrarySourceReadFailure(
  "viewer_source_superseded",
  "The original-image stream no longer belongs to the visible selection",
);
