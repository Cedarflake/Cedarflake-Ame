import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_states.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_viewport.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_review_toolbar.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_transition.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_review_fixture.dart";

void main() {
  for (final scale in [1.0, 2.0]) {
    testWidgets(
      "empty content follows visible center and review actions stay right at ${scale}x text",
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(1600, 900));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final expanded = ValueNotifier(true);
        final widths = <double>[];
        final mode = LibraryOrganizationMode(
          ReviewServiceFixture(),
          ReviewSurfaceFixture(),
        );
        addTearDown(expanded.dispose);
        addTearDown(mode.dispose);
        await mode.enter();
        var imports = 0;
        await tester.pumpWidget(
          _fixture(
            expanded: expanded,
            mode: mode,
            widths: widths,
            scale: scale,
            onImport: () => imports++,
          ),
        );
        await tester.pumpAndSettle();
        final title = find.text("建立你的图片图库");
        final toolbar = find.byKey(const Key("library-review-toolbar"));
        final boundary = find.byKey(const Key("library-gallery-transition"));
        void expectAligned() {
          final center = tester.getRect(boundary).center.dx;
          expect(tester.getCenter(title).dx, closeTo(center, .01));
          expect(
            tester.getRect(toolbar).right,
            closeTo(tester.getRect(boundary).right, .01),
          );
          expect(tester.takeException(), isNull);
        }

        expectAligned();
        for (final value in [false, true, false]) {
          final before = tester.getCenter(title);
          expanded.value = value;
          await tester.pump();
          expect(tester.getCenter(title), before);
          expectAligned();
          await tester.pump(const Duration(milliseconds: 60));
          expectAligned();
        }
        await tester.pumpAndSettle();
        expectAligned();
        expect(widths, [1340, 1600, 1340, 1600]);
        await tester.tap(find.widgetWithText(FilledButton, "导入"));
        expect(imports, 1);
      },
    );
  }

  testWidgets("reduced motion places centered content immediately", (
    tester,
  ) async {
    final expanded = ValueNotifier(true);
    final mode = LibraryOrganizationMode(
      ReviewServiceFixture(),
      ReviewSurfaceFixture(),
    );
    addTearDown(expanded.dispose);
    addTearDown(mode.dispose);
    await mode.enter();
    await tester.pumpWidget(
      _fixture(
        expanded: expanded,
        mode: mode,
        widths: [],
        reducedMotion: true,
        onImport: () {},
      ),
    );
    await tester.pumpAndSettle();
    expanded.value = false;
    await tester.pump();
    expect(tester.getCenter(find.text("建立你的图片图库")).dx, 400);
    await tester.pump();
    expect(tester.binding.transientCallbackCount, 0);
    expect(tester.takeException(), isNull);
  });
}

Widget _fixture({
  required ValueNotifier<bool> expanded,
  required LibraryOrganizationMode mode,
  required List<double> widths,
  required VoidCallback onImport,
  double scale = 1,
  bool reducedMotion = false,
}) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(context).copyWith(
      textScaler: TextScaler.linear(scale),
      disableAnimations: reducedMotion,
    ),
    child: child!,
  ),
  home: Scaffold(
    body: ValueListenableBuilder(
      valueListenable: expanded,
      builder: (_, value, _) => LibrarySidebarTransition(
        expanded: value,
        sidebarWidth: 260,
        sidebar: const Text("Folders"),
        resizeHandle: const SizedBox.shrink(),
        gallery: LayoutBuilder(
          builder: (_, constraints) {
            widths.add(constraints.maxWidth);
            return LibraryGalleryViewport(
              header: SizedBox(
                height: 80,
                child: Align(
                  alignment: Alignment.centerRight,
                  child: LibraryReviewToolbar(mode: mode),
                ),
              ),
              body: EmptyLibraryGallery(
                state: const LibraryState(),
                onImport: onImport,
              ),
              isEmpty: true,
              isLoading: false,
            );
          },
        ),
      ),
    ),
  ),
);
