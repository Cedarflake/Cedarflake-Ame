import "dart:io";

import "package:flutter/painting.dart";

ImageProvider<Object> createLibraryPreviewImage(String previewPath) =>
    FileImage(File(previewPath));
