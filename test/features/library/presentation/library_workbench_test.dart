import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_main_surface.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/application_test_bindings.dart";

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
    final container = ProviderContainer(overrides: applicationTestOverrides());
    addTearDown(container.dispose);
    final settingsWidths = <double>[];
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: LibraryWorkbench(
              globalBarBuilder: (_, _, toggle) => SizedBox(
                height: 64,
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: SizedBox(width: 72, child: Center(child: toggle)),
                ),
              ),
              albumsNavigationBuilder: (_) =>
                  ListTile(title: const Text("Album list"), onTap: () {}),
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
              settingsBuilder: (_) => LayoutBuilder(
                builder: (_, constraints) {
                  settingsWidths.add(constraints.maxWidth);
                  return const TextField(key: Key("settings-input"));
                },
              ),
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
    final surface = find.byType(LibraryMainSurface);
    final galleryBounds = tester.getRect(surface);
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pump();
    expect(tester.getRect(surface), galleryBounds);
    final settingsLayoutCount = settingsWidths.length;
    await tester.pump(const Duration(milliseconds: 60));
    expect(settingsWidths.length, settingsLayoutCount);
    final intermediate = tester.getRect(surface);
    expect(intermediate.left, inExclusiveRange(80, galleryBounds.left));
    expect(intermediate.right, galleryBounds.right);
    expect(intermediate.top, galleryBounds.top);
    expect(intermediate.bottom, galleryBounds.bottom);
    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pump();
    expect(tester.getRect(surface), intermediate);
    await tester.pumpAndSettle();
    expect(tester.getRect(surface), galleryBounds);
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pumpAndSettle();
    expect(tester.getRect(surface).left, 80);
    expect(tester.getRect(surface).right, 1272);
    expect(tester.getRect(surface).bottom, 792);
    expect(settingsWidths.toSet(), {1192});
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
    expect(tester.getRect(surface).left, 80);
    expect(tester.getRect(surface).right, 1272);
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
    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pumpAndSettle();
    for (final expanding in [false, true]) {
      await tester.tap(find.byKey(const Key("library-sidebar-toggle")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 60));
      final interrupted = tester.getRect(surface);
      expect(interrupted.left, inExclusiveRange(80, galleryBounds.left));

      await tester.tap(find.byKey(const Key("library-sidebar-settings")));
      await tester.pump();
      expect(tester.getRect(surface), interrupted);
      final layouts = settingsWidths.length;
      await tester.pump(const Duration(milliseconds: 60));
      final returning = tester.getRect(surface);
      expect(returning.left, inExclusiveRange(80, interrupted.left));
      expect(settingsWidths.length, layouts);

      await tester.tap(find.byKey(const Key("workbench-gallery")));
      await tester.pump();
      expect(tester.getRect(surface), returning);
      await tester.pumpAndSettle();
      expect(tester.getRect(surface).left, expanding ? galleryBounds.left : 80);
      expect(tester.getRect(surface).right, galleryBounds.right);
      expect(identical(scroll.position, originalPosition), isTrue);
      expect(scroll.offset, 1200);
    }
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
        overrides: applicationTestOverrides(),
        child: MaterialApp(
          home: Scaffold(
            body: LibraryWorkbench(
              globalBarBuilder: (_, _, toggle) => SizedBox(
                height: 64,
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: SizedBox(width: 72, child: Center(child: toggle)),
                ),
              ),
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
