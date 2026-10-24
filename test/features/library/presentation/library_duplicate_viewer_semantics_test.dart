import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_asset_information_sheet.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/vertical_material_timeline_slider.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  for (final reducedMotion in [false, true]) {
    testWidgets(
      "duplicate preview retains semantics through return and resizing (reduced=$reducedMotion)",
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(1280, 800));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final semantics = tester.ensureSemantics();
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer(location: const Offset(10, 10));
        try {
          await tester.pumpWidget(
            ProviderScope(
              overrides: [
                libraryControllerProvider.overrideWith(
                  () => ReviewFixtureLibrary(
                    duplicateMode: LibraryGalleryDuplicateMode.merged,
                  ),
                ),
                libraryGalleryDuplicateAnnotationsProvider.overrideWith(
                  (_) async => {"one": reviewGroup},
                ),
                libraryExactDuplicatesProvider.overrideWithValue(_Duplicates()),
                libraryDuplicateReviewProvider.overrideWithValue(
                  ReviewFixtureService(),
                ),
              ],
              child: MaterialApp(
                builder: (context, child) => MediaQuery(
                  data: MediaQuery.of(
                    context,
                  ).copyWith(disableAnimations: reducedMotion),
                  child: child!,
                ),
                home: Scaffold(
                  body: LibraryWorkbench(
                    albumsNavigationBuilder: (_) => const Text("Album list"),
                    navigationBuilder: (width) => SizedBox(
                      width: width,
                      child: ListView(
                        children: const [ListTile(title: Text("Folders"))],
                      ),
                    ),
                    galleryBuilder: (context) => Row(
                      children: [
                        Expanded(
                          child: ListView(
                            children: [
                              LibraryPhotoTile(
                                asset: reviewAsset("one"),
                                width: 240,
                                height: 160,
                                isSelecting: false,
                                isSelected: false,
                                onOpen: (_) {},
                                onToggleSelection: (_) {},
                                onViewInformation: (asset) => unawaited(
                                  showLibraryAssetInformation(context, asset),
                                ),
                                onCopyPath: (_) {},
                                onRevealFile: (_) {},
                              ),
                            ],
                          ),
                        ),
                        SizedBox(
                          width: 48,
                          child: VerticalMaterialTimelineSlider(
                            value: 0.35,
                            endpointInset: 24,
                            onChanged: (_) {},
                            onChangeStart: (_) {},
                            onChangeEnd: (_) {},
                            semanticLabelFor: (_) => "2026年3月",
                          ),
                        ),
                      ],
                    ),
                    duplicatesBuilder: (_) =>
                        const LibraryExactDuplicatesPage(),
                    settingsBuilder: (_) => const Text("Settings"),
                    onSidebarResizeChanged: (_) {},
                    onPreferenceError: (_, error) => fail("$error"),
                  ),
                ),
              ),
            ),
          );
          await tester.pumpAndSettle();
          final toggle = find.byKey(const Key("library-sidebar-toggle"));
          final badge = find.byKey(const ValueKey("duplicate-badge-one"));
          for (var cycle = 0; cycle < 3; cycle += 1) {
            await tester.tap(find.byKey(const Key("workbench-duplicates")));
            await tester.pumpAndSettle();
            if (cycle == 0) {
              await tester.tap(
                find.byKey(const ValueKey("duplicate-card-one")),
              );
              await tester.pumpAndSettle();
            }
            await tester.tap(find.byKey(const ValueKey("duplicate-card-one")));
            await tester.pumpAndSettle();
            expect(find.byType(LibraryImageViewer), findsOneWidget);
            expect(
              tester.semantics.simulatedAccessibilityTraversal().map(
                (node) => node.getSemanticsData().value,
              ),
              contains("100%"),
            );
            RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
              trace: "duplicate-viewer-open-$cycle",
            );
            await tester.tap(find.byKey(const Key("viewer-information")));
            await tester.pumpAndSettle();
            await tester.sendKeyEvent(LogicalKeyboardKey.escape);
            await tester.pumpAndSettle();
            expect(find.byType(LibraryImageViewer), findsOneWidget);
            await tester.binding.setSurfaceSize(const Size(1920, 1080));
            await tester.pumpAndSettle();
            await tester.binding.setSurfaceSize(const Size(1280, 800));
            await tester.pumpAndSettle();
            await tester.sendKeyEvent(LogicalKeyboardKey.escape);
            await tester.pumpAndSettle();
            await tester.binding.setSurfaceSize(const Size(1920, 1080));
            await tester.pumpAndSettle();
            await tester.binding.setSurfaceSize(const Size(1280, 800));
            await tester.pumpAndSettle();
            await tester.tap(find.byKey(const Key("workbench-gallery")));
            await tester.pumpAndSettle();
            await mouse.moveTo(tester.getCenter(badge));
            await tester.pump(const Duration(milliseconds: 700));
            await tester.tap(badge);
            await tester.pumpAndSettle();
            RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
              trace: "duplicate-dialog-open-$cycle",
            );
            await tester.sendKeyEvent(LogicalKeyboardKey.escape);
            await tester.pumpAndSettle();
            await mouse.moveTo(tester.getCenter(toggle));
            await tester.pump(const Duration(milliseconds: 700));
            await tester.binding.setSurfaceSize(const Size(1920, 1080));
            await tester.pumpAndSettle();
            await tester.binding.setSurfaceSize(const Size(1280, 800));
            await tester.pumpAndSettle();
            await tester.tap(
              find.byType(LibraryPhotoTile),
              buttons: kSecondaryMouseButton,
            );
            await tester.pumpAndSettle();
            await tester.tap(find.text(LibraryStrings.viewInformation));
            await tester.pumpAndSettle();
            RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
              trace: "information-open-$cycle",
            );
            await tester.sendKeyEvent(LogicalKeyboardKey.escape);
            await tester.pumpAndSettle();
            for (var tap = 0; tap < 12; tap += 1) {
              await tester.tap(toggle);
              await tester.pump();
              await tester.pump(const Duration(milliseconds: 35));
              RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
                trace: "sidebar-$cycle-$tap",
              );
            }
            await tester.binding.setSurfaceSize(
              cycle.isEven ? const Size(1920, 1080) : const Size(1280, 800),
            );
            await tester.pumpAndSettle();
            expect(tester.takeException(), isNull);
          }
        } finally {
          await mouse.removePointer();
          semantics.dispose();
        }
      },
      variant: TargetPlatformVariant.only(TargetPlatform.windows),
    );
  }
}

class _Duplicates extends Fake implements LibraryExactDuplicates {
  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async => const LibraryDuplicateLocationPage(locations: []);
}
