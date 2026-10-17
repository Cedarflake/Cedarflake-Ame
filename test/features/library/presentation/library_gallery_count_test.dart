import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final mode in LibraryGalleryDuplicateMode.values) {
    for (final width in [800.0, 1280.0]) {
      testWidgets("gallery keeps the source count in $mode at $width", (
        tester,
      ) async {
        await tester.binding.setSurfaceSize(Size(width, 800));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final merged = mode == LibraryGalleryDuplicateMode.merged;
        final state = LibraryState(
          status: LibraryStatus.completed,
          query: LibraryGalleryQuery(duplicateMode: mode),
          queryId: "counts",
          catalogRevision: BigInt.one,
          roots: const [
            LibraryRoot(
              id: "root",
              path: "C:\\Fixture",
              displayPath: "C:\\Fixture",
              createdUnixMs: 1,
              assetCount: 10000,
              issueCount: 0,
            ),
          ],
          timeline: LibraryTimeline(
            revision: BigInt.one,
            queryId: "counts",
            totalItems: merged ? 72 : 10000,
            totalSourceItems: 10000,
            buckets: const [],
          ),
        );
        await tester.pumpWidget(
          ProviderScope(
            overrides: [initialLibraryStateProvider.overrideWithValue(state)],
            child: const AmeApp(),
          ),
        );
        await tester.pump();
        final summary = tester.widget<Text>(
          find.byKey(const Key("library-summary")),
        );
        expect(summary.data, merged ? "10000 张图片 · 去重后 72 张" : "10000 张图片");
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }
}
