import "package:cedarflake_ame/features/library/presentation/widgets/library_folder_disclosure.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flutter/rendering.dart";
import "package:material_ui/material_ui.dart";

void main() {
  testWidgets("selection ink stays inside the opening and closing branch", (
    tester,
  ) async {
    final expanded = ValueNotifier(false);
    final boundaryKey = GlobalKey();
    addTearDown(expanded.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: RepaintBoundary(
              key: boundaryKey,
              child: SizedBox(
                width: 200,
                height: 250,
                child: Material(
                  color: Colors.white,
                  child: ValueListenableBuilder(
                    valueListenable: expanded,
                    builder: (context, value, _) => Column(
                      children: [
                        const SizedBox(height: 50),
                        LibraryFolderDisclosure(
                          child: value
                              ? const SizedBox(
                                  height: 100,
                                  child: ListTile(
                                    selected: true,
                                    selectedTileColor: Colors.blue,
                                    title: Text("Child"),
                                  ),
                                )
                              : null,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expanded.value = true;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));
    await _expectWhitePixel(tester, boundaryKey, 10, 90);
    await tester.pumpAndSettle();
    expanded.value = false;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 80));
    await _expectWhitePixel(tester, boundaryKey, 10, 90);
    await tester.pumpAndSettle();
    expect(find.text("Child"), findsNothing);
  });

  testWidgets("folder children reverse in place and retire closing input", (
    tester,
  ) async {
    final expanded = ValueNotifier(false);
    final reduced = ValueNotifier(false);
    final childFocus = FocusNode();
    addTearDown(expanded.dispose);
    addTearDown(reduced.dispose);
    addTearDown(childFocus.dispose);
    var selected = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ListenableBuilder(
            listenable: Listenable.merge([expanded, reduced]),
            builder: (context, _) => MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(disableAnimations: reduced.value),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  LibraryFolderDisclosure(
                    child: expanded.value
                        ? SizedBox(
                            height: 100,
                            child: TextButton(
                              focusNode: childFocus,
                              onPressed: () => selected++,
                              child: const Text("Child folder"),
                            ),
                          )
                        : null,
                  ),
                  const SizedBox(key: Key("next-folder"), height: 40),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    double nextTop() =>
        tester.getTopLeft(find.byKey(const Key("next-folder"))).dy;
    expect(find.text("Child folder"), findsNothing);
    expanded.value = true;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    final openingTop = nextTop();
    expect(openingTop, greaterThan(0));
    expect(openingTop, lessThan(100));
    expanded.value = false;
    await tester.pump();
    expect(nextTop(), closeTo(openingTop, .01));
    expect(find.text("Child folder"), findsOneWidget);
    expect(childFocus.canRequestFocus, isFalse);
    await tester.tap(find.text("Child folder"), warnIfMissed: false);
    expect(selected, 0);
    await tester.pump(const Duration(milliseconds: 16));
    final closingTop = nextTop();
    expanded.value = true;
    await tester.pump();
    expect(nextTop(), closeTo(closingTop, .01));
    await tester.pumpAndSettle();
    expect(nextTop(), 100);
    await tester.tap(find.text("Child folder"));
    expect(selected, 1);
    expanded.value = false;
    await tester.pumpAndSettle();
    expect(nextTop(), 0);
    expect(find.text("Child folder"), findsNothing);

    reduced.value = true;
    expanded.value = true;
    await tester.pump();
    expect(nextTop(), 100);
    expanded.value = false;
    await tester.pump();
    expect(find.text("Child folder"), findsNothing);
    expect(tester.binding.transientCallbackCount, 0);
  });
}

Future<void> _expectWhitePixel(
  WidgetTester tester,
  GlobalKey boundaryKey,
  int x,
  int y,
) async {
  final boundary =
      boundaryKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final pixel = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = (await image.toByteData())!;
      final offset = (y * image.width + x) * 4;
      return [
        for (var index = 0; index < 3; index++) data.getUint8(offset + index),
      ];
    } finally {
      image.dispose();
    }
  });
  expect(pixel, [
    255,
    255,
    255,
  ], reason: "A clipped child cannot paint selection ink outside its reveal");
}
