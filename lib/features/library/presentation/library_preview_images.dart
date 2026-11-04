import "package:flutter/painting.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

typedef LibraryPreviewImages =
    ImageProvider<Object> Function(String previewPath);

final libraryPreviewImagesProvider = Provider<LibraryPreviewImages>(
  (ref) => throw StateError("Library preview image binding is required"),
);
