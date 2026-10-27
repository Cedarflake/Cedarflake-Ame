import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_main_surface.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation_resize_handle.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("shows the unified empty library shell", (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    for (final key in const [
      Key("library-function-rail"),
      Key("library-sidebar-library"),
      Key("library-gallery-header"),
    ]) {
      expect(
        find.descendant(of: find.byKey(key), matching: find.text("图库")),
        findsOneWidget,
      );
    }
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
    expect(find.text("Read-only validation"), findsNothing);
    expect(find.byKey(const Key("library-task-activity-button")), findsNothing);
    expect(find.byKey(const Key("library-import-button")), findsNothing);
    expect(find.byKey(const Key("library-sidebar-import")), findsOneWidget);
    expect(find.byKey(const Key("library-sidebar-settings")), findsOneWidget);
    expect(find.text("Cedarflake Ame"), findsOneWidget);
    expect(find.text("Ame"), findsNothing);
    expect(
      find.ancestor(
        of: find.byKey(const Key("window-close")),
        matching: find.byKey(const Key("library-global-bar")),
      ),
      findsOneWidget,
    );
    final searchCenter = tester.getCenter(
      find.byKey(const Key("library-search")),
    );
    final globalBarCenter = tester.getCenter(
      find.byKey(const Key("library-global-bar")),
    );
    final closeCenter = tester.getCenter(find.byKey(const Key("window-close")));
    final minimizeCenter = tester.getCenter(
      find.byKey(const Key("window-minimize")),
    );
    final notificationCenter = tester.getCenter(
      find.byKey(const Key("notification-history-button")),
    );
    expect(searchCenter.dy, closeTo(globalBarCenter.dy, 0.1));
    expect(tester.getSize(find.byKey(const Key("library-search"))).height, 44);
    expect(closeCenter.dy, closeTo(globalBarCenter.dy, 0.1));
    expect(notificationCenter.dx, lessThan(minimizeCenter.dx));
    expect(
      notificationCenter.dx,
      greaterThan(
        tester.getTopRight(find.byKey(const Key("library-search"))).dx,
      ),
    );
    expect(find.byKey(const Key("notification-read-icon")), findsOneWidget);
    expect(find.text("通知历史"), findsNothing);
    expect(
      find.ancestor(
        of: find.byKey(const Key("library-search")),
        matching: find.byKey(const Key("library-global-bar")),
      ),
      findsOneWidget,
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

  testWidgets("pins desktop caption controls to the maximized right edge", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(2048, 1200);
    tester.view.devicePixelRatio = 1;
    tester.view.padding = const FakeViewPadding(top: 8, right: 24);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPadding);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    expect(
      tester.getTopLeft(find.byKey(const Key("library-global-bar"))).dy,
      0,
    );
    expect(tester.getTopRight(find.byKey(const Key("window-close"))).dx, 2040);
  });

  testWidgets("separates the application shell with Material surfaces", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pump();

    final context = tester.element(
      find.byKey(const Key("library-main-surface")),
    );
    final colorScheme = Theme.of(context).colorScheme;
    final scaffold = tester.widget<Scaffold>(find.byType(Scaffold).first);
    final globalSurface = tester.widget<Material>(
      find.byKey(const Key("library-global-surface")),
    );
    final navigationSurface = tester.widget<Material>(
      find.byKey(const Key("library-navigation-surface")),
    );
    final mainSurface = tester.widget<Material>(
      find.byKey(const Key("library-main-surface")),
    );

    expect(scaffold.backgroundColor, colorScheme.surfaceContainerLow);
    expect(globalSurface.color, colorScheme.surfaceContainerHigh);
    expect(navigationSurface.color, colorScheme.surfaceContainerLow);
    expect(mainSurface.type, MaterialType.transparency);
    expect(mainSurface.clipBehavior, Clip.none);
    final backgrounds = find.ancestor(
      of: find.byKey(const Key("library-main-surface")),
      matching: find.byType(DecoratedBox),
    );
    final backgroundBox = tester.widget<DecoratedBox>(backgrounds.first);
    final background = backgroundBox.decoration as BoxDecoration;
    expect(background.color, colorScheme.surfaceContainerLowest);
    expect(background.borderRadius, LibraryMainSurface.borderRadius);
    final galleryClips = find.ancestor(
      of: find.byKey(const Key("library-main-surface")),
      matching: find.byType(ClipRRect),
    );
    expect(galleryClips, findsOneWidget);
    expect(
      tester.widget<ClipRRect>(galleryClips).borderRadius,
      LibraryMainSurface.borderRadius,
    );
    expect(
      tester
          .widgetList<Material>(
            find.ancestor(
              of: find.byKey(const Key("library-main-surface")),
              matching: find.byType(Material),
            ),
          )
          .every((surface) => surface.clipBehavior == Clip.none),
      isTrue,
    );
    expect(
      find.ancestor(
        of: find.byKey(const Key("library-gallery-header")),
        matching: find.byKey(const Key("library-main-surface")),
      ),
      findsOneWidget,
    );
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
        overrides: [
          initialAmePreferencesProvider.overrideWithValue(preferences),
          amePreferenceStoreProvider.overrideWithValue(preferenceStore),
        ],
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

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.text("Cedarflake Ame"), findsOneWidget);
    expect(find.text("Ame"), findsNothing);
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
    expect(galleryHeader.right, 1100);
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
