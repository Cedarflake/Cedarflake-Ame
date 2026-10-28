import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_selection_icon.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "only top-level icons select; all and removal omit child folders",
    (tester) async {
      final removed = <String>[];
      var navigated = "";
      final roots = [
        for (final id in ["one", "two"])
          LibraryRoot(
            id: id,
            path: "C:\\$id",
            displayPath: "C:\\$id",
            createdUnixMs: 0,
            assetCount: 1,
            issueCount: 0,
          ),
      ];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LibraryNavigation(
              isCompact: false,
              width: 300,
              roots: roots,
              selectedRootId: null,
              selectedFolderRelativePath: null,
              transientSource: null,
              folderTree: LibraryFolderTreeState(
                branches: {
                  const LibraryFolderBranchKey(
                    rootId: "one",
                    parentRelativePath: "",
                  ): const LibraryFolderBranch(
                    hasLoaded: true,
                    folders: [
                      LibraryFolder(
                        rootId: "one",
                        relativePath: "child",
                        name: "Child",
                        directAssetCount: 1,
                        descendantAssetCount: 1,
                      ),
                    ],
                  ),
                },
              ),
              isBusy: false,
              isBrowseDisabled: false,
              onSelectLibrary: () {},
              onSelectRoot: (root) => navigated = root.id,
              onSelectFolder: (_, folder) => navigated = folder.relativePath,
              onExpandFolder: (_, _) async {},
              onLoadMoreFolders: (_, _) async {},
              onAddSource: () {},
              onUpdateRoot: (_) {},
              onOpenRoot: (_) {},
              onOpenFolder: (_, _) {},
              onRemoveRoot: (_) {},
              onRemoveRoots: (selected) async {
                removed.addAll(selected.map((root) => root.id));
                return {};
              },
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const ValueKey("source-expand-one")));
      await tester.pumpAndSettle();
      expect(find.byType(LibrarySidebarSelectionIcon), findsNWidgets(2));
      await tester.tap(
        find.byKey(const ValueKey("library-sidebar-select-root-one")),
      );
      await tester.pumpAndSettle();
      expect(find.text("已选择 1"), findsOneWidget);
      expect(navigated, isEmpty);
      await tester.tap(find.text("Child"));
      expect(navigated, "child");
      await tester.tap(find.byKey(const Key("library-sidebar-select-all")));
      await tester.pumpAndSettle();
      expect(find.text("已选择 2"), findsOneWidget);
      await tester.tap(
        find.byKey(const Key("library-sidebar-remove-selected")),
      );
      await tester.pumpAndSettle();
      expect(removed, ["one", "two"]);
      await tester.tap(find.byKey(const Key("library-sidebar-select-all")));
      await tester.pumpAndSettle();
      expect(find.text("文件夹"), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey("source-title-two")));
      expect(navigated, "two");
      await tester.tap(
        find.byKey(const ValueKey("library-sidebar-select-root-two")),
      );
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(find.text("文件夹"), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}
