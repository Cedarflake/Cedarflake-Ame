import "dart:async";
import "dart:ui" as ui;

import "package:flutter/foundation.dart";
import "package:flutter/painting.dart";

import "../../application/library_source_read_scheduler.dart";
import "../../application/library_source_reader.dart";
import "../../application/rust_library_source_reader.dart";
import "../../domain/library_models.dart";
import "library_source_image_stream.dart";

typedef LibrarySourceBufferLoader =
    Future<ui.ImmutableBuffer> Function(String path);

class LibrarySourceImage extends ImageProvider<LibrarySourceImage> {
  LibrarySourceImage(
    LibraryAsset asset, {
    this.retryGeneration = 0,
    LibrarySourceReadScheduler? scheduler,
    LibrarySourceBufferLoader? bufferLoader,
  }) : _asset = asset,
       _scheduler = scheduler ?? librarySourceReadScheduler,
       _bufferLoader = bufferLoader ?? ui.ImmutableBuffer.fromFilePath,
       sourceVersion = (
         rootId: asset.rootId,
         scanId: asset.activeScanId,
         locationId: asset.locationId,
         path: asset.sourcePath,
         fileSize: asset.fileSize,
         modifiedUnixMs: asset.modifiedUnixMs,
         identityScheme: asset.fileIdentity?.scheme,
         identityValue: asset.fileIdentity?.value,
         revision: asset.sourceRevision,
         generation: asset.sourceGeneration,
       );

  final LibraryAsset _asset;
  final LibrarySourceReadScheduler _scheduler;
  final LibrarySourceBufferLoader _bufferLoader;
  final _SourceImageReadLifetime _lifetime = _SourceImageReadLifetime();
  final ({
    String rootId,
    String scanId,
    String locationId,
    String path,
    BigInt fileSize,
    int modifiedUnixMs,
    String? identityScheme,
    String? identityValue,
    LibrarySourceRevisionEvidence? revision,
    BigInt generation,
  })
  sourceVersion;
  final int retryGeneration;

  void cancel() => _lifetime.cancel();

  @override
  Future<LibrarySourceImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(
    LibrarySourceImage key,
    ImageDecoderCallback decode,
  ) => createLibrarySourceImageStream(
    codec: _load(decode),
    isRetired: () => _lifetime.isCancelled,
    debugLabel: _asset.locationId,
  );

  Future<ui.Codec> _load(ImageDecoderCallback decode) async {
    try {
      _lifetime.checkCurrent();
      final request = _scheduler.request(_asset);
      _lifetime.request = request;
      final lease = await request.lease;
      ui.ImmutableBuffer? buffer;
      try {
        try {
          _lifetime.checkCurrent();
          buffer = await _bufferLoader(lease.sourcePath);
        } finally {
          // Flutter owns a copy when fromFilePath completes; only then can the
          // native namespace/source guard be released, even after disposal.
          await lease.close();
        }
        _lifetime.checkCurrent();
      } on Object {
        buffer?.dispose();
        rethrow;
      }
      return await decode(buffer);
    } on Object {
      _lifetime.checkCurrent();
      scheduleMicrotask(() {
        if (!_lifetime.isCancelled) {
          PaintingBinding.instance.imageCache.evict(this);
        }
      });
      rethrow;
    }
  }

  @override
  bool operator ==(Object other) =>
      other is LibrarySourceImage &&
      identical(_scheduler, other._scheduler) &&
      _bufferLoader == other._bufferLoader &&
      sourceVersion == other.sourceVersion &&
      retryGeneration == other.retryGeneration;

  @override
  int get hashCode =>
      Object.hash(_scheduler, _bufferLoader, sourceVersion, retryGeneration);
}

class _SourceImageReadLifetime {
  LibrarySourceReadRequest? request;
  bool _cancelled = false;

  bool get isCancelled => _cancelled || (request?.isCancelled ?? false);

  void cancel() {
    _cancelled = true;
    request?.cancel();
  }

  void checkCurrent() {
    if (isCancelled) {
      throw const LibrarySourceReadFailure(
        "viewer_source_superseded",
        "The selected original-image source is no longer visible",
      );
    }
  }
}
