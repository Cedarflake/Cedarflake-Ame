import "package:flutter_riverpod/flutter_riverpod.dart";

enum AmeThemePreference { system, light, dark }

enum ImageViewerWheelBehavior { zoom, previousOrNext }

enum ImageViewerOpenBehavior { automatic, fitWindow, actualSize }

enum PreviewLoadingSpeed { small, medium, large }

const ameDefaultSidebarWidth = 260.0;
const ameMinimumSidebarWidth = 220.0;
const ameMaximumSidebarWidth = 420.0;

class AmePreferences {
  const AmePreferences({
    this.theme = AmeThemePreference.system,
    this.viewerWheelBehavior = ImageViewerWheelBehavior.zoom,
    this.viewerOpenBehavior = ImageViewerOpenBehavior.automatic,
    this.viewerFilmstripVisible = false,
    this.previewLoadingSpeed = PreviewLoadingSpeed.medium,
    this.sidebarWidth = ameDefaultSidebarWidth,
    this.sidebarExpanded,
  });

  final AmeThemePreference theme;
  final ImageViewerWheelBehavior viewerWheelBehavior;
  final ImageViewerOpenBehavior viewerOpenBehavior;
  final bool viewerFilmstripVisible;
  final PreviewLoadingSpeed previewLoadingSpeed;
  final double sidebarWidth;
  final bool? sidebarExpanded;

  AmePreferences copyWith({
    AmeThemePreference? theme,
    ImageViewerWheelBehavior? viewerWheelBehavior,
    ImageViewerOpenBehavior? viewerOpenBehavior,
    bool? viewerFilmstripVisible,
    PreviewLoadingSpeed? previewLoadingSpeed,
    double? sidebarWidth,
    bool? sidebarExpanded,
  }) {
    return AmePreferences(
      theme: theme ?? this.theme,
      viewerWheelBehavior: viewerWheelBehavior ?? this.viewerWheelBehavior,
      viewerOpenBehavior: viewerOpenBehavior ?? this.viewerOpenBehavior,
      viewerFilmstripVisible:
          viewerFilmstripVisible ?? this.viewerFilmstripVisible,
      previewLoadingSpeed: previewLoadingSpeed ?? this.previewLoadingSpeed,
      sidebarWidth: sidebarWidth ?? this.sidebarWidth,
      sidebarExpanded: sidebarExpanded ?? this.sidebarExpanded,
    );
  }
}

abstract interface class AmePreferenceStore {
  Future<AmePreferences> loadAmePreferences();

  Future<void> saveAmePreferences(AmePreferences preferences);
}

final initialAmePreferencesProvider = Provider<AmePreferences>(
  (ref) => const AmePreferences(),
);

final amePreferenceStoreProvider = Provider<AmePreferenceStore>(
  (ref) => _VolatileAmePreferenceStore(),
);

final amePreferencesControllerProvider =
    NotifierProvider<AmePreferencesController, AmePreferences>(
      AmePreferencesController.new,
    );

class AmePreferencesController extends Notifier<AmePreferences> {
  @override
  AmePreferences build() => ref.watch(initialAmePreferencesProvider);

  Future<void> update(AmePreferences preferences) async {
    final previous = state;
    state = preferences;
    try {
      await ref
          .read(amePreferenceStoreProvider)
          .saveAmePreferences(preferences);
    } on Object {
      if (identical(state, preferences)) {
        state = previous;
      }
      rethrow;
    }
  }
}

class _VolatileAmePreferenceStore implements AmePreferenceStore {
  AmePreferences _preferences = const AmePreferences();

  @override
  Future<AmePreferences> loadAmePreferences() async => _preferences;

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    _preferences = preferences;
  }
}
