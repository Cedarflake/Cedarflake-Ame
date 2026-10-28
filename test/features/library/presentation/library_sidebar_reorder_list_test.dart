import "dart:async";

import "package:cedarflake_ame/features/library/application/library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_reorder_list.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
        itemBuilder: (_, item, index, draggable) =>
            ReorderableDragStartListener(
              index: index,
              enabled: draggable,
              child: ListTile(
                title: Text(item),
                onTap: () {},
                minTileHeight: 60,
              ),
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
