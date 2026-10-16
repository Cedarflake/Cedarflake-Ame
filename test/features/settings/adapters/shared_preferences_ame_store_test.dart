import "package:cedarflake_ame/features/settings/adapters/shared_preferences_ame_store.dart";
import "package:cedarflake_ame/app/window/ame_window_placement.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter_test/flutter_test.dart";
import "package:shared_preferences/shared_preferences.dart";
import "package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart";
import "package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart";

void main() {
  test(
    "defaults to automatic sizing and preserves all explicit opening preferences",
    () async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.empty();
      final preferences = SharedPreferencesAsync();
      final store = SharedPreferencesAmeStore(preferences: preferences);
      expect(
        (await store.loadAmePreferences()).viewerOpenBehavior,
        ImageViewerOpenBehavior.automatic,
      );
      for (final behavior in ImageViewerOpenBehavior.values) {
        await store.saveAmePreferences(
          AmePreferences(viewerOpenBehavior: behavior),
        );
        expect((await store.loadAmePreferences()).viewerOpenBehavior, behavior);
      }
    },
  );

  test("round-trips window, gallery, and presentation preferences", () async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.empty();
    final preferences = SharedPreferencesAsync();
    final store = SharedPreferencesAmeStore(preferences: preferences);
    const placement = AmeWindowPlacement(
      left: -1200,
      top: 80,
      width: 1440,
      height: 900,
      isMaximized: true,
    );
    const gallery = LibraryViewPreferences(
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.large,
      sortKey: LibraryGallerySortKey.fileName,
      sortDirection: LibraryGallerySortDirection.ascending,
    );
    const presentation = AmePreferences(
      viewerFilmstripVisible: true,
      theme: AmeThemePreference.dark,
      viewerWheelBehavior: ImageViewerWheelBehavior.previousOrNext,
      viewerOpenBehavior: ImageViewerOpenBehavior.actualSize,
      previewLoadingSpeed: PreviewLoadingSpeed.large,
      sidebarWidth: 348,
      sidebarExpanded: false,
    );

    await store.saveWindowPlacement(placement);
    await store.saveLibraryViewPreferences(gallery);
    await store.saveAmePreferences(presentation);

    final restoredPlacement = await store.loadWindowPlacement();
    final restoredGallery = await store.loadLibraryViewPreferences();
    final restoredPresentation = await store.loadAmePreferences();
    expect(restoredPlacement?.left, placement.left);
    expect(restoredPlacement?.top, placement.top);
    expect(restoredPlacement?.width, placement.width);
    expect(restoredPlacement?.height, placement.height);
    expect(restoredPlacement?.isMaximized, isTrue);
    expect(restoredGallery.layoutShape, gallery.layoutShape);
    expect(restoredGallery.thumbnailSize, gallery.thumbnailSize);
    expect(restoredGallery.sortKey, gallery.sortKey);
    expect(restoredGallery.sortDirection, gallery.sortDirection);
    expect(restoredPresentation.theme, presentation.theme);
    expect(
      restoredPresentation.viewerWheelBehavior,
      presentation.viewerWheelBehavior,
    );
    expect(
      restoredPresentation.viewerOpenBehavior,
      presentation.viewerOpenBehavior,
    );
    expect(
      restoredPresentation.previewLoadingSpeed,
      presentation.previewLoadingSpeed,
    );
    expect(restoredPresentation.sidebarWidth, presentation.sidebarWidth);
    expect(restoredPresentation.sidebarExpanded, isFalse);
    expect(restoredPresentation.viewerFilmstripVisible, isTrue);
  });

  test("falls back safely when stored preferences are malformed", () async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.empty();
    final preferences = SharedPreferencesAsync();
    await preferences.setString("ame.window.placement.v1", "{not-json");
    await preferences.setString(
      "ame.library.view.v1",
      '{"version":1,"layoutShape":"removed-layout"}',
    );
    await preferences.setString("ame.presentation.v1", "{not-json");
    final store = SharedPreferencesAmeStore(preferences: preferences);

    expect(await store.loadWindowPlacement(), isNull);
    final gallery = await store.loadLibraryViewPreferences();
    expect(gallery.layoutShape, GalleryLayoutShape.equalHeight);
    expect(gallery.thumbnailSize, GalleryThumbnailSize.medium);
    expect(gallery.sortKey, LibraryGallerySortKey.captureTime);
    expect(gallery.sortDirection, LibraryGallerySortDirection.descending);
    expect(
      await store.loadAmePreferences(),
      isA<AmePreferences>()
          .having((value) => value.theme, "theme", AmeThemePreference.system)
          .having(
            (value) => value.viewerWheelBehavior,
            "viewerWheelBehavior",
            ImageViewerWheelBehavior.zoom,
          )
          .having(
            (value) => value.viewerOpenBehavior,
            "viewerOpenBehavior",
            ImageViewerOpenBehavior.automatic,
          )
          .having(
            (value) => value.previewLoadingSpeed,
            "previewLoadingSpeed",
            PreviewLoadingSpeed.medium,
          ),
    );
  });

  test(
    "older preferences keep the filmstrip hidden and preserve opening mode",
    () async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.empty();
      final preferences = SharedPreferencesAsync();
      await preferences.setString(
        "ame.presentation.v1",
        '{"version":1,"viewerOpenBehavior":"actualSize"}',
      );
      final store = SharedPreferencesAmeStore(preferences: preferences);
      final restored = await store.loadAmePreferences();
      expect(restored.viewerFilmstripVisible, isFalse);
      await store.saveAmePreferences(
        restored.copyWith(viewerFilmstripVisible: true),
      );
      final updated = await store.loadAmePreferences();
      expect(updated.viewerFilmstripVisible, isTrue);
      expect(updated.viewerOpenBehavior, ImageViewerOpenBehavior.actualSize);
    },
  );
}
