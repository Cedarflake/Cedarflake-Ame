import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation_resize_handle.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ports/ame_preference_store.dart";
import "package:cedarflake_ame/features/settings/domain/ame_preferences.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../../support/application_test_bindings.dart";

void main() {
  testWidgets("opens settings and returns to the empty library", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: applicationTestOverrides(),
        child: const AmeApp(),
      ),
    );

    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pump();
    expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.byKey(const Key("library-global-bar")), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(find.byKey(const Key("library-sidebar-settings")))
          .isSelected,
      isTrue,
    );

    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pump();
    expect(find.byKey(const Key("ame-settings-page")), findsNothing);
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
  });

  testWidgets("resizes, resets, and persists the expanded sidebar width", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    const preferences = AmePreferences(sidebarWidth: 300);
    final preferenceStore = _RecordingAmePreferenceStore(preferences);

    await tester.pumpWidget(
      ProviderScope(
        overrides: applicationTestOverrides([
          initialAmePreferencesProvider.overrideWithValue(preferences),
          amePreferenceStoreProvider.overrideWithValue(preferenceStore),
        ]),
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      300,
    );
    final resizeHandle = find.byKey(const Key("library-sidebar-resize-handle"));
    final navigation = find.byKey(const Key("library-navigation"));
    final mainSurface = find.byKey(const Key("library-main-surface"));
    final boundaryX = tester.getTopRight(navigation).dx;
    expect(tester.getTopLeft(mainSurface).dx, boundaryX);
    expect(
      tester.getSize(resizeHandle).width,
      LibraryNavigationResizeHandle.hitTargetWidth,
    );
    expect(tester.getCenter(resizeHandle).dx, boundaryX);

    await tester.drag(resizeHandle, const Offset(64, 0));
    await tester.pump();

    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      closeTo(364, 1),
    );
    expect(preferenceStore.saved.last.sidebarWidth, closeTo(364, 1));
    expect(
      find.descendant(
        of: resizeHandle,
        matching: find.byType(AnimatedContainer),
      ),
      findsNothing,
    );

    final handleCenter = tester.getCenter(resizeHandle);
    await tester.tapAt(handleCenter);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(handleCenter);
    await tester.pump(const Duration(milliseconds: 100));
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameDefaultSidebarWidth,
    );
    expect(preferenceStore.saved.last.sidebarWidth, ameDefaultSidebarWidth);

    final anchoredHandleCenter = tester.getCenter(resizeHandle);
    final gesture = await tester.startGesture(anchoredHandleCenter);
    await gesture.moveBy(const Offset(240, 0));
    await tester.pump();
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameMaximumSidebarWidth,
    );

    await gesture.moveBy(const Offset(-40, 0));
    await tester.pump();
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameMaximumSidebarWidth,
    );

    await gesture.moveBy(const Offset(-80, 0));
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 100));
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      closeTo(380, 1),
    );
    expect(preferenceStore.saved.last.sidebarWidth, closeTo(380, 1));
  });

  testWidgets("keeps the gallery actions visible at the desktop minimum", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1100, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: applicationTestOverrides(),
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.text("Ame"), findsOneWidget);
    expect(find.byKey(const Key("library-search")), findsOneWidget);
    expect(find.byKey(const Key("library-import-button")), findsNothing);
    expect(find.byKey(const Key("library-sidebar-import")), findsOneWidget);
    expect(find.byKey(const Key("library-sidebar-settings")), findsOneWidget);
    expect(find.byKey(const Key("window-minimize")), findsOneWidget);
    expect(find.byKey(const Key("window-maximize")), findsOneWidget);
    expect(find.byKey(const Key("window-close")), findsOneWidget);
    final galleryHeader = tester.getRect(
      find.byKey(const Key("library-gallery-header")),
    );
    final galleryTitle = tester.getRect(
      find.byKey(const Key("library-gallery-title")),
    );
    final sidebar = tester.getRect(find.byKey(const Key("library-navigation")));
    final toggle = tester.getRect(
      find.byKey(const Key("library-sidebar-toggle")),
    );
    expect(galleryHeader.left, closeTo(sidebar.right, 0.1));
    expect(galleryHeader.right, 1092);
    for (final key in const [
      Key("library-select-button"),
      Key("library-sort-menu"),
      Key("library-layout-menu"),
      Key("library-more-menu"),
    ]) {
      expect(find.byKey(key).hitTestable(), findsOneWidget);
    }
    expect(galleryTitle.left, closeTo(galleryHeader.left + 28, 0.1));
    final settings = tester.getRect(
      find.byKey(const Key("library-sidebar-settings")),
    );
    expect(toggle.center.dx, settings.center.dx);
    expect(toggle.bottom, lessThanOrEqualTo(settings.top));
  });
}

class _RecordingAmePreferenceStore implements AmePreferenceStore {
  _RecordingAmePreferenceStore(this.initialPreferences);

  final AmePreferences initialPreferences;
  final List<AmePreferences> saved = [];

  @override
  Future<AmePreferences> loadAmePreferences() async => initialPreferences;

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    saved.add(preferences);
  }
}
