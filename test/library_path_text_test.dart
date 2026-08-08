import "package:cedarflake_ame/features/library/presentation/widgets/library_path_text.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("extracts a file name from an adapter-provided display path", () {
    expect(
      displayLibraryFileName(r"C:\Users\Ame\Pictures\very-long-name.png"),
      "very-long-name.png",
    );
  });

  testWidgets("shows the readable full path when a label is truncated", (
    tester,
  ) async {
    const readablePath = r"C:\Users\Example\Documents";
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 48,
              child: LibraryPathText(
                text: "Documents",
                path: readablePath,
                textKey: Key("truncated-path"),
              ),
            ),
          ),
        ),
      ),
    );

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    await mouse.moveTo(
      tester.getCenter(find.byKey(const Key("truncated-path"))),
    );
    await tester.pump(const Duration(milliseconds: 600));

    expect(find.text(readablePath), findsOneWidget);
  });

  testWidgets("does not rewrite the backend-provided display path", (
    tester,
  ) async {
    const backendPath = r"\\?\C:\Pictures\sample.png";
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: LibraryPathText(text: "sample.png", path: backendPath),
        ),
      ),
    );

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer();
    await mouse.moveTo(tester.getCenter(find.text("sample.png")));
    await tester.pump(const Duration(milliseconds: 600));

    expect(find.text(backendPath), findsOneWidget);
  });
}
