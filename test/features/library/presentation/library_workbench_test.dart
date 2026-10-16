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
              navigationBuilder: (width) => SizedBox(
                key: const Key("folders"),
                width: width,
                child: const TextField(),
              ),
              galleryBuilder: (toggle) => Column(
                children: [
                  toggle,
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
    scroll.jumpTo(1200);
    await tester.pump();
    final originalPosition = scroll.position;
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
    expect(find.byTooltip("展开侧栏"), findsOneWidget);
    expect(
      container.read(amePreferencesControllerProvider).sidebarExpanded,
      isFalse,
    );
    await tester.tap(find.byTooltip("展开侧栏"));
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
              navigationBuilder: (width) =>
                  SizedBox(width: width, child: const Text("Folder tree")),
              galleryBuilder: (toggle) =>
                  Align(alignment: Alignment.topLeft, child: toggle),
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
    await tester.tap(find.byTooltip("展开侧栏"));
    await tester.pumpAndSettle();
    expect(find.text("Folder tree"), findsOneWidget);
    expect(find.byTooltip("收起侧栏"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
