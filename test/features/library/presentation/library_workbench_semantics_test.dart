import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_workbench.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/settings_section.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets(
    "retained Settings restores semantics after closing its menu",
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Scaffold(
                body: LibraryWorkbench(
                  navigationBuilder: (width) =>
                      SizedBox(width: width, child: const Text("Folders")),
                  galleryBuilder: (toggle) =>
                      Column(children: [toggle, const Text("Gallery")]),
                  duplicatesBuilder: (_) => const Text("Duplicates"),
                  settingsBuilder: (_) => Column(
                    children: [
                      const Text("设置"),
                      SettingsChoice<int>(
                        value: 0,
                        entries: const [
                          SettingsChoiceEntry(value: 0, label: "跟随系统"),
                          SettingsChoiceEntry(value: 1, label: "浅色"),
                          SettingsChoiceEntry(value: 2, label: "深色"),
                        ],
                        onSelected: (_) {},
                      ),
                    ],
                  ),
                  onSidebarResizeChanged: (_) {},
                  onPreferenceError: (_, error) => fail("$error"),
                ),
              ),
            ),
          ),
        );
        for (var cycle = 0; cycle < 3; cycle += 1) {
          await tester.tap(find.byKey(const Key("library-sidebar-settings")));
          await tester.pumpAndSettle();
          await tester.tap(find.byType(OutlinedButton));
          await tester.pumpAndSettle();
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "settings-open-$cycle",
          );
          await tester.sendKeyEvent(LogicalKeyboardKey.escape);
          await tester.pumpAndSettle();
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "settings-closed-$cycle",
          );
          expect(
            tester.semantics.simulatedAccessibilityTraversal().map(
              (node) => node.getSemanticsData().label,
            ),
            contains("设置"),
          );
          await tester.tap(find.byKey(const Key("workbench-gallery")));
          await tester.pumpAndSettle();
        }
      } finally {
        semantics.dispose();
      }
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );

  testWidgets(
    "gallery badges retain valid semantics when publication changes membership",
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final folded = ValueNotifier(false);
        addTearDown(folded.dispose);
        var groups = <String, LibraryDuplicateGroup>{};
        final container = ProviderContainer(
          overrides: [
            libraryGalleryDuplicateAnnotationsProvider.overrideWith(
              (ref) async => groups,
            ),
          ],
        );
        addTearDown(container.dispose);
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: MaterialApp(
              home: Scaffold(
                body: ValueListenableBuilder(
                  valueListenable: folded,
                  builder: (_, merged, _) => ListView(
                    children: [
                      Text(merged ? "1 张图片" : "2 张图片"),
                      Row(children: [_photo(0), if (!merged) _photo(1)]),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
        for (var cycle = 0; cycle < 3; cycle += 1) {
          groups = {
            for (var index = 0; index < 2; index += 1)
              "location-$index": const LibraryDuplicateGroup(
                digest:
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                byteLength: 20,
                locationCount: 2,
                physicalFileCount: 2,
                representativeLocationId: "location-0",
                representativePath: r"C:\fixture\0.jpg",
              ),
          };
          container.invalidate(libraryGalleryDuplicateAnnotationsProvider);
          folded.value = true;
          await tester.pump();
          await tester.pump();
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "badges-folded-$cycle",
          );
          groups = {};
          container.invalidate(libraryGalleryDuplicateAnnotationsProvider);
          folded.value = false;
          await tester.pump();
          await tester.pump();
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "badges-expanded-$cycle",
          );
        }
      } finally {
        semantics.dispose();
      }
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );
}

Widget _photo(int index) => LibraryPhotoTile(
  key: ValueKey(index),
  asset: LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "root",
    activeScanId: "scan",
    sourceRevision: null,
    sourceGeneration: BigInt.from(index + 1),
    sourcePath: "C:\\fixture\\$index.jpg",
    displayPath: "C:\\fixture\\$index.jpg",
    relativePath: "$index.jpg",
    previewPath: "",
    fileSize: BigInt.from(20),
    modifiedUnixMs: 1,
    width: 4,
    height: 3,
    previewStatus: LibraryPreviewStatus.pending,
  ),
  width: 240,
  height: 180,
  isSelecting: false,
  isSelected: false,
  onOpen: (_) {},
  onToggleSelection: (_) {},
  onViewInformation: (_) {},
  onCopyPath: (_) {},
  onRevealFile: (_) {},
);
