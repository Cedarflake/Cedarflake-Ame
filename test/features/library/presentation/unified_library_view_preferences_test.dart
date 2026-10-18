import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "support/library_screen_fixture.dart";
import "support/library_screen_menu_fixture.dart";

void main() {
  testWidgets("restores and saves stable gallery display preferences", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = populatedScreenState(totalItems: 1);
    final catalog = RecordingQueryCatalog(
      LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: "preferences-result",
        roots: initialState.roots,
        assets: initialState.assets,
      ),
    );
    const initialPreferences = LibraryViewPreferences(
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.large,
    );
    final preferenceStore = _RecordingViewPreferenceStore(initialPreferences);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          initialLibraryViewPreferencesProvider.overrideWithValue(
            initialPreferences,
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryViewPreferenceStoreProvider.overrideWithValue(preferenceStore),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final layoutMenuButton = find.byKey(const Key("library-layout-menu"));
    await tester.tap(layoutMenuButton);
    await tester.pumpAndSettle();
    expect(find.byType(AmeMenuItemContent), findsNWidgets(5));
    expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.square,
      icon: Symbols.grid_view_rounded,
    );
    expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.large,
      icon: Symbols.crop_square_rounded,
    );
    await tester.tapAt(tester.getCenter(layoutMenuButton));
    await tester.pumpAndSettle();
    expect(find.text(LibraryStrings.equalHeight), findsNothing);

    await tester.tap(layoutMenuButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.equalHeight));
    await tester.pumpAndSettle();
    await tester.tap(layoutMenuButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.small));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("library-sort-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.fileName).last);
    await tester.pumpAndSettle();

    expect(preferenceStore.saved, isNotEmpty);
    expect(
      preferenceStore.saved.last.layoutShape,
      GalleryLayoutShape.equalHeight,
    );
    expect(
      preferenceStore.saved.last.thumbnailSize,
      GalleryThumbnailSize.small,
    );
    expect(preferenceStore.saved.last.sortKey, LibraryGallerySortKey.fileName);
  });
}

class _RecordingViewPreferenceStore implements LibraryViewPreferenceStore {
  _RecordingViewPreferenceStore(this.initialPreferences);

  final LibraryViewPreferences initialPreferences;
  final List<LibraryViewPreferences> saved = [];

  @override
  Future<LibraryViewPreferences> loadLibraryViewPreferences() async {
    return initialPreferences;
  }

  @override
  Future<void> saveLibraryViewPreferences(
    LibraryViewPreferences preferences,
  ) async {
    saved.add(preferences);
  }
}
