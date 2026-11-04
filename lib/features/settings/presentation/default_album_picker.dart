import "package:flutter/widgets.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

typedef DefaultAlbumSelection = ({String? albumId});
typedef DefaultAlbumPicker =
    Future<DefaultAlbumSelection?> Function(
      BuildContext context,
      String? initialAlbumId,
    );

final defaultAlbumPickerProvider = Provider<DefaultAlbumPicker>(
  (ref) => throw StateError("Default album picker binding is required"),
);
