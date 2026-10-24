import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("switches workspaces without rebuilding gallery state", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: LibraryWorkbench(
              albumsNavigationBuilder: (_) => const Text("Album list"),
              navigationBuilder: (width) => SizedBox(
                key: const Key("folders"),
                width: width,
                child: const TextField(),
              ),
              galleryBuilder: (_) => Column(
                children: [
                  Expanded(
                    child: ListView.builder(
                      key: const Key("gallery"),
                      controller: scroll,
                      itemExtent: 80,
                      itemCount: 200,
                      itemBuilder: (_, index) => Text("Image $index"),
                    ),
                  ),
                ],
              ),
              duplicatesBuilder: (_) => const Text("Duplicate results"),
              settingsBuilder: (_) =>
                  const TextField(key: Key("settings-input")),
              onSidebarResizeChanged: (_) {},
              onPreferenceError: (_, error) => fail("$error"),
            ),
          ),
        ),
      ),
    );
    await tester.enterText(find.byType(TextField), "Expanded folder state");
    final toggleBounds = tester.getRect(
      find.byKey(const Key("library-sidebar-toggle")),
    );
    final settingsBounds = tester.getRect(
      find.byKey(const Key("library-sidebar-settings")),
    );
    expect(toggleBounds.center.dx, settingsBounds.center.dx);
    expect(toggleBounds.bottom, lessThanOrEqualTo(settingsBounds.top));
    scroll.jumpTo(1200);
    await tester.pump();
    final originalPosition = scroll.position;
    final rail = tester.widget<NavigationRail>(
      find.byKey(const Key("library-function-rail")),
    );
    expect(
      rail.destinations.map((destination) => (destination.label as Text).data),
      ["图库", "相册", "去重"],
    );
    await tester.tap(find.byKey(const Key("workbench-albums")));
    await tester.pumpAndSettle();
    expect(find.text("Album list"), findsOneWidget);
    expect(find.byKey(const Key("folders")), findsNothing);
    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pumpAndSettle();
    expect(find.text("Album list"), findsNothing);
    expect(find.text("Expanded folder state"), findsOneWidget);
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key("settings-input")),
      "Retained setting",
    );
    await tester.tap(find.byKey(const Key("workbench-duplicates")));
    await tester.pumpAndSettle();
    expect(find.text("Duplicate results"), findsOneWidget);
    expect(find.byKey(const Key("gallery")), findsNothing);
    expect(
      tester
          .widget<EditableText>(
            find.byType(EditableText, skipOffstage: false).first,
          )
          .focusNode
          .hasFocus,
      isFalse,
    );
    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pumpAndSettle();
    expect(identical(scroll.position, originalPosition), isTrue);
    expect(scroll.offset, 1200);
    expect(find.text("Expanded folder state"), findsOneWidget);
    await tester.tap(find.byKey(const Key("library-sidebar-toggle")));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key("folders")), findsNothing);
    expect(
      find.byWidgetPredicate(
        (widget) => widget is AmeTooltip && widget.message == "展开侧栏",
      ),
      findsOneWidget,
    );
    expect(
      container.read(amePreferencesControllerProvider).sidebarExpanded,
      isFalse,
    );
    await tester.tap(find.byKey(const Key("library-sidebar-toggle")));
    await tester.pumpAndSettle();
    expect(find.text("Expanded folder state"), findsOneWidget);
    expect(scroll.offset, 1200);
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pumpAndSettle();
    expect(find.text("Retained setting"), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("a narrow window keeps the folder toggle reachable", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 560);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: LibraryWorkbench(
              albumsNavigationBuilder: (_) => const Text("Album list"),
              navigationBuilder: (width) =>
                  SizedBox(width: width, child: const Text("Folder tree")),
              galleryBuilder: (_) => const Text("Gallery"),
              duplicatesBuilder: (_) => const Text("Duplicates"),
              settingsBuilder: (_) => const Text("Settings"),
              onSidebarResizeChanged: (_) {},
              onPreferenceError: (_, error) => fail("$error"),
            ),
          ),
        ),
      ),
    );
    expect(find.text("Folder tree"), findsNothing);
    await tester.tap(find.byKey(const Key("library-sidebar-toggle")));
    await tester.pumpAndSettle();
    expect(find.text("Folder tree"), findsOneWidget);
    expect(
      find.byWidgetPredicate(
        (widget) => widget is AmeTooltip && widget.message == "收起侧栏",
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}
