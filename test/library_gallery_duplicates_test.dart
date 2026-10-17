import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "gallery preference defaults to merged and query identity includes mode",
    () {
      expect(
        const LibraryViewPreferences().duplicateMode,
        LibraryGalleryDuplicateMode.merged,
      );
      const all = LibraryGalleryQuery();
      expect(
        all,
        isNot(all.copyWith(duplicateMode: LibraryGalleryDuplicateMode.merged)),
      );
    },
  );

  testWidgets("filter changes only its chosen query dimension", (tester) async {
    var query = const LibraryGalleryQuery(
      rootId: "root",
      folderRelativePath: "Album",
      searchText: "cat",
      sortKey: LibraryGallerySortKey.fileName,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => LibraryGalleryFilter(
              query: query,
              onChanged: (value) => setState(() => query = value),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key("library-filter-menu")));
    await tester.pumpAndSettle();
    await tester.tap(
      find.ancestor(
        of: find.text("合并重复图片"),
        matching: find.byWidgetPredicate(
          (widget) => widget is PopupMenuItem<Object?>,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(query.duplicateMode, LibraryGalleryDuplicateMode.merged);
    expect(query.rootId, "root");
    expect(query.searchText, "cat");
    expect(query.sortKey, LibraryGallerySortKey.fileName);
    await tester.tap(find.byKey(const Key("library-filter-menu")));
    await tester.pumpAndSettle();
    await tester.tap(
      find.ancestor(
        of: find.text("隐藏子目录"),
        matching: find.byWidgetPredicate(
          (widget) => widget is PopupMenuItem<Object?>,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(query.includeDescendants, isFalse);
    expect(query.duplicateMode, LibraryGalleryDuplicateMode.merged);
    expect(query.folderRelativePath, "Album");
  });

  test("annotation batches are shared and bounded to 256 locations", () async {
    final service = _Annotations();
    final container = ProviderContainer(
      overrides: [
        libraryControllerProvider.overrideWith(() => _Window(_state(513, 1))),
        libraryGalleryDuplicatesProvider.overrideWithValue(service),
      ],
    );
    addTearDown(container.dispose);
    final subscription = container.listen(
      libraryGalleryDuplicateAnnotationsProvider,
      (_, _) {},
    );
    addTearDown(subscription.close);
    await container.read(libraryGalleryDuplicateAnnotationsProvider.future);
    expect(service.sizes, [256, 256, 1]);
    container.read(libraryGalleryDuplicateAnnotationsProvider);
    await container.pump();
    expect(service.sizes, [256, 256, 1]);
  });

  test(
    "retired annotation response cannot overwrite a newer catalog window",
    () async {
      final service = _Annotations()..delayed = Completer();
      final container = ProviderContainer(
        overrides: [
          libraryControllerProvider.overrideWith(() => _Window(_state(513, 1))),
          libraryGalleryDuplicatesProvider.overrideWithValue(service),
        ],
      );
      addTearDown(container.dispose);
      final subscription = container.listen(
        libraryGalleryDuplicateAnnotationsProvider,
        (_, _) {},
      );
      addTearDown(subscription.close);
      await container.pump();
      final old = service.delayed!;
      service.delayed = null;
      (container.read(libraryControllerProvider.notifier) as _Window).replace(
        _state(1, 2),
      );
      await container.pump();
      await container.read(libraryGalleryDuplicateAnnotationsProvider.future);
      old.complete({"stale": _group});
      await container.pump();
      expect(
        container
            .read(libraryGalleryDuplicateAnnotationsProvider)
            .asData
            ?.value,
        isEmpty,
      );
      expect(service.sizes, [256, 1]);
    },
  );
}

const _group = LibraryDuplicateGroup(
  digest: "digest",
  byteLength: 20,
  locationCount: 2,
  physicalFileCount: 2,
  representativeLocationId: "a",
  representativePath: "a.png",
);

class _Annotations implements LibraryGalleryDuplicates {
  final sizes = <int>[];
  Completer<Map<String, LibraryDuplicateGroup>>? delayed;

  @override
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  ) async {
    sizes.add(locationIds.length);
    if (delayed case final pending?) {
      return pending.future;
    }
    return {};
  }
}

class _Window extends LibraryController {
  _Window(this.initial);
  final LibraryState initial;
  @override
  LibraryState build() => initial;
  void replace(LibraryState value) => state = value;
}

LibraryState _state(int count, int revision) => LibraryState(
  catalogRevision: BigInt.from(revision),
  assets: [
    for (var index = 0; index < count; index++)
      LibraryAsset(
        assetId: "$index",
        locationId: "$index",
        rootId: "root",
        activeScanId: "scan",
        sourcePath: "$index.png",
        displayPath: "$index.png",
        relativePath: "$index.png",
        previewPath: "",
        fileSize: BigInt.from(20),
        modifiedUnixMs: 1,
        sourceRevision: null,
        sourceGeneration: BigInt.one,
        width: 100,
        height: 100,
      ),
  ],
);
