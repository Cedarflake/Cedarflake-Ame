import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_scroll_motion_theme.dart";
import "package:cedarflake_ame/features/library/domain/library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_folder_reorder_item.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_reorder_list.dart";
import "package:flutter/gestures.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("expanded folder drag stays anchored inside the sidebar", (
    tester,
  ) async {
    final writes = <LibrarySidebarMove>[];
    final expanded = ValueNotifier(true);
    var selections = 0;
    var menuClicks = 0;
    addTearDown(expanded.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: AmeScrollMotionTheme(
          child: Scaffold(
            body: Stack(
              children: [
                Positioned(
                  left: 80,
                  top: 64,
                  width: 260,
                  height: 400,
                  child: ValueListenableBuilder(
                    valueListenable: expanded,
                    builder: (context, value, _) =>
                        LibrarySidebarReorderList<String>(
                          items: const ["folder"],
                          itemId: (item) => item,
                          header: (_) => const SizedBox(height: 80),
                          onReorder: (move) async => writes.add(move),
                          itemBuilder: (_, item, index, draggable) =>
                              LibraryFolderReorderItem(
                                index: index,
                                enabled: draggable,
                                onCollapse: () => expanded.value = false,
                                header: ListTile(
                                  title: Text(item),
                                  onTap: () => selections++,
                                  trailing: IconButton(
                                    key: const Key("folder-more"),
                                    icon: const Icon(Icons.more_vert),
                                    onPressed: () => menuClicks++,
                                  ),
                                ),
                                child: value
                                    ? Column(
                                        children: List.generate(
                                          12,
                                          (index) => ListTile(
                                            title: Text("child $index"),
                                          ),
                                        ),
                                      )
                                    : null,
                              ),
                        ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final row = find.text("folder");
    final before = tester.getTopLeft(row);
    for (final target in [row, find.byKey(const Key("folder-more"))]) {
      final click = await tester.startGesture(
        tester.getCenter(target),
        kind: PointerDeviceKind.mouse,
      );
      await click.moveBy(const Offset(4, 3));
      await tester.pump();
      expect(expanded.value, isTrue);
      await click.up();
      await tester.pumpAndSettle();
      expect(expanded.value, isTrue);
      expect(find.text("child 0"), findsOneWidget);
    }
    expect(selections, 1);
    expect(menuClicks, 1);
    expect(writes, isEmpty);
    final pointer = await tester.startGesture(
      tester.getCenter(row),
      kind: PointerDeviceKind.mouse,
    );
    await pointer.moveBy(const Offset(0, 24));
    await tester.pump();
    await tester.pump();
    expect(expanded.value, isFalse);
    expect(tester.getTopLeft(row).dx, closeTo(before.dx, 0.1));
    expect(tester.getTopLeft(row).dy, greaterThanOrEqualTo(before.dy));
    expect(find.text("child 0"), findsNothing);
    await pointer.up();
    await tester.pumpAndSettle();
    expect(writes, isEmpty);
    expect(tester.getTopLeft(row).dx, closeTo(before.dx, 0.1));
    expect(tester.takeException(), isNull);

    expanded.value = true;
    await tester.pumpAndSettle();
    final quickDrag = await tester.startGesture(
      tester.getCenter(row),
      kind: PointerDeviceKind.mouse,
    );
    await quickDrag.moveBy(const Offset(0, 24));
    await quickDrag.up();
    await tester.pumpAndSettle();
    expect(find.text("child 0"), findsNothing);
    expect(writes, isEmpty);
    expanded.value = true;
    await tester.pumpAndSettle();
    expect(find.text("child 0"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "a real drag saves relative identities and preserves the new order",
    (tester) async {
      final writes = <LibrarySidebarMove>[];
      await tester.pumpWidget(
        _list(["a", "b", "c"], (move) async => writes.add(move)),
      );
      await _drag(tester, "a", "c");
      expect(writes.single.entryId, "a");
      expect(writes.single.anchorId, "c");
      expect(writes.single.placement, LibrarySidebarPlacement.after);
      expect(
        tester.getTopLeft(find.text("a")).dy,
        greaterThan(tester.getTopLeft(find.text("c")).dy),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("replacing an item during a drag cannot save stale indexes", (
    tester,
  ) async {
    final writes = <LibrarySidebarMove>[];
    Future<void> save(LibrarySidebarMove move) async => writes.add(move);
    await tester.pumpWidget(_list(["a", "b", "c"], save));
    final pointer = await tester.startGesture(
      tester.getCenter(find.text("a")),
      kind: PointerDeviceKind.mouse,
    );
    await pointer.moveBy(const Offset(0, 30));
    await tester.pump();
    await tester.pumpWidget(_list(["d", "b", "c"], save));
    await pointer.moveTo(tester.getCenter(find.text("c")));
    await pointer.up();
    await tester.pumpAndSettle();
    expect(writes, isEmpty);
    expect(find.text("a"), findsNothing);
    expect(find.text("d"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets("failure rolls back and uses the existing transient feedback", (
    tester,
  ) async {
    final save = Completer<void>();
    await tester.pumpWidget(_list(["a", "b", "c"], (_) => save.future));
    await _drag(tester, "a", "c");
    expect(
      tester.getTopLeft(find.text("a")).dy,
      greaterThan(tester.getTopLeft(find.text("c")).dy),
    );
    save.completeError(StateError("write failed"));
    await tester.pumpAndSettle();
    expect(
      tester.getTopLeft(find.text("a")).dy,
      lessThan(tester.getTopLeft(find.text("b")).dy),
    );
    expect(find.text("排列顺序未能保存，请重新拖动。"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

Widget _list(
  List<String> items,
  Future<void> Function(LibrarySidebarMove) save,
) => MaterialApp(
  home: Scaffold(
    body: SizedBox(
      width: 260,
      height: 400,
      child: LibrarySidebarReorderList<String>(
        items: items,
        itemId: (item) => item,
        header: (_) => const Text("相册"),
        onReorder: save,
        itemBuilder: (_, item, index, draggable) => LibraryFolderReorderItem(
          index: index,
          enabled: draggable,
          onCollapse: () {},
          child: null,
          header: ListTile(title: Text(item), onTap: () {}, minTileHeight: 60),
        ),
      ),
    ),
  ),
);

Future<void> _drag(WidgetTester tester, String from, String to) async {
  final pointer = await tester.startGesture(
    tester.getCenter(find.text(from)),
    kind: PointerDeviceKind.mouse,
  );
  await pointer.moveBy(const Offset(0, 25));
  await tester.pump();
  await pointer.moveTo(tester.getCenter(find.text(to)) + const Offset(0, 80));
  await tester.pump(const Duration(milliseconds: 250));
  await pointer.up();
  await tester.pumpAndSettle();
}
