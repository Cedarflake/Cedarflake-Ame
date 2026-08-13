import "dart:convert";

import "package:shared_preferences/shared_preferences.dart";

import "../../../app/window/ame_window_placement.dart";
import "../../library/application/library_view_preferences.dart";
import "../../library/domain/library_models.dart";
import "../application/ame_preferences.dart";

class SharedPreferencesAmeStore
    implements
        AmePreferenceStore,
        AmeWindowPreferenceStore,
        LibraryViewPreferenceStore {
  SharedPreferencesAmeStore({SharedPreferencesAsync? preferences})
    : _preferences = preferences ?? SharedPreferencesAsync();

  static const _libraryViewKey = "ame.library.view.v1";
  static const _presentationKey = "ame.presentation.v1";
  static const _windowPlacementKey = "ame.window.placement.v1";

  final SharedPreferencesAsync _preferences;
  Future<void> _writeQueue = Future<void>.value();

  @override
  Future<LibraryViewPreferences> loadLibraryViewPreferences() async {
    String? encoded;
    try {
      encoded = await _preferences.getString(_libraryViewKey);
    } on Object {
      return const LibraryViewPreferences();
    }
    final value = _decodeObject(encoded);
    if (value == null || value["version"] != 1) {
      return const LibraryViewPreferences();
    }
    return LibraryViewPreferences(
      layoutShape: _enumByName(
        GalleryLayoutShape.values,
        value["layoutShape"],
        GalleryLayoutShape.equalHeight,
      ),
      thumbnailSize: _enumByName(
        GalleryThumbnailSize.values,
        value["thumbnailSize"],
        GalleryThumbnailSize.medium,
      ),
      sortKey: _enumByName(
        LibraryGallerySortKey.values,
        value["sortKey"],
        LibraryGallerySortKey.captureTime,
      ),
      sortDirection: _enumByName(
        LibraryGallerySortDirection.values,
        value["sortDirection"],
        LibraryGallerySortDirection.descending,
      ),
    );
  }

  @override
  Future<AmePreferences> loadAmePreferences() async {
    String? encoded;
    try {
      encoded = await _preferences.getString(_presentationKey);
    } on Object {
      return const AmePreferences();
    }
    final value = _decodeObject(encoded);
    if (value == null || value["version"] != 1) {
      return const AmePreferences();
    }
    return AmePreferences(
      theme: _enumByName(
        AmeThemePreference.values,
        value["theme"],
        AmeThemePreference.system,
      ),
      viewerWheelBehavior: _enumByName(
        ImageViewerWheelBehavior.values,
        value["viewerWheelBehavior"],
        ImageViewerWheelBehavior.zoom,
      ),
      viewerOpenBehavior: _enumByName(
        ImageViewerOpenBehavior.values,
        value["viewerOpenBehavior"],
        ImageViewerOpenBehavior.fitWindow,
      ),
      previewLoadingSpeed: _enumByName(
        PreviewLoadingSpeed.values,
        value["previewLoadingSpeed"],
        PreviewLoadingSpeed.medium,
      ),
      sidebarWidth:
          (_readDouble(value["sidebarWidth"]) ?? ameDefaultSidebarWidth)
              .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
              .toDouble(),
    );
  }

  @override
  Future<AmeWindowPlacement?> loadWindowPlacement() async {
    String? encoded;
    try {
      encoded = await _preferences.getString(_windowPlacementKey);
    } on Object {
      return null;
    }
    final value = _decodeObject(encoded);
    if (value == null || value["version"] != 1) {
      return null;
    }
    final left = _readDouble(value["left"]);
    final top = _readDouble(value["top"]);
    final width = _readDouble(value["width"]);
    final height = _readDouble(value["height"]);
    final isMaximized = value["isMaximized"];
    if (left == null ||
        top == null ||
        width == null ||
        height == null ||
        isMaximized is! bool) {
      return null;
    }
    return AmeWindowPlacement(
      left: left,
      top: top,
      width: width,
      height: height,
      isMaximized: isMaximized,
    );
  }

  @override
  Future<void> saveLibraryViewPreferences(LibraryViewPreferences preferences) {
    return _write(
      _libraryViewKey,
      jsonEncode({
        "version": 1,
        "layoutShape": preferences.layoutShape.name,
        "thumbnailSize": preferences.thumbnailSize.name,
        "sortKey": preferences.sortKey.name,
        "sortDirection": preferences.sortDirection.name,
      }),
    );
  }

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) {
    return _write(
      _presentationKey,
      jsonEncode({
        "version": 1,
        "theme": preferences.theme.name,
        "viewerWheelBehavior": preferences.viewerWheelBehavior.name,
        "viewerOpenBehavior": preferences.viewerOpenBehavior.name,
        "previewLoadingSpeed": preferences.previewLoadingSpeed.name,
        "sidebarWidth": preferences.sidebarWidth,
      }),
    );
  }

  @override
  Future<void> saveWindowPlacement(AmeWindowPlacement placement) {
    return _write(
      _windowPlacementKey,
      jsonEncode({
        "version": 1,
        "left": placement.left,
        "top": placement.top,
        "width": placement.width,
        "height": placement.height,
        "isMaximized": placement.isMaximized,
      }),
    );
  }

  Future<void> _write(String key, String value) {
    final operation = _writeQueue.then(
      (_) => _preferences.setString(key, value),
    );
    _writeQueue = operation.then<void>((_) {}, onError: (_) {});
    return operation;
  }

  static Map<String, Object?>? _decodeObject(String? encoded) {
    if (encoded == null) {
      return null;
    }
    try {
      final decoded = jsonDecode(encoded);
      if (decoded is! Map) {
        return null;
      }
      return decoded.map((key, value) => MapEntry(key.toString(), value));
    } on FormatException {
      return null;
    }
  }

  static double? _readDouble(Object? value) {
    if (value is! num) {
      return null;
    }
    final result = value.toDouble();
    return result.isFinite ? result : null;
  }

  static T _enumByName<T extends Enum>(
    List<T> values,
    Object? name,
    T fallback,
  ) {
    if (name is! String) {
      return fallback;
    }
    for (final value in values) {
      if (value.name == name) {
        return value;
      }
    }
    return fallback;
  }
}
