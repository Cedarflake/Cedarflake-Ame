import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_cards.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_grid.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  for (final width in [400.0, 1280.0]) {
    for (final scale in [1.0, 2.0]) {
      testWidgets("review cards and actions fit at $width and ${scale}x text", (
        tester,
      ) async {
        await tester.binding.setSurfaceSize(Size(width, 900));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final service = _PagedReview();
        final analysis = LibraryExactAnalysisController(_NoAnalysis());
        final scheduler = LibraryExactAnalysisScheduler(analysis);
        final container = ProviderContainer(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryDuplicateReviewProvider.overrideWithValue(service),
            libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
            libraryExactAnalysisSchedulerProvider.overrideWithValue(scheduler),
          ],
        );
        addTearDown(() {
          container.dispose();
          scheduler.dispose();
          analysis.dispose();
        });
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: MaterialApp(
              builder: (context, child) => MediaQuery(
                data: MediaQuery.of(
                  context,
                ).copyWith(textScaler: TextScaler.linear(scale)),
                child: child!,
              ),
              home: const Scaffold(body: LibraryExactDuplicatesPage()),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text("重复图片查找"), findsOneWidget);
        expect(find.text("72 个重复图像组"), findsOneWidget);
        expect(find.textContaining("范围："), findsNothing);
        expect(find.text("查找内容完全相同的图片"), findsNothing);
        expect(find.text("查找完成"), findsNothing);
        final cards = find.byType(LibraryDuplicateGroupCard);
        if (width > 1000) {
          expect(
            tester.getTopLeft(cards.at(0)).dy,
            tester.getTopLeft(cards.at(1)).dy,
          );
          expect(
            tester.getTopLeft(cards.at(1)).dx,
            greaterThan(tester.getTopLeft(cards.at(0)).dx),
          );
          expect(
            tester.getTopLeft(find.text("暂停查重")).dy,
            tester.getTopLeft(find.text("刷新结果")).dy,
          );
        }
        await tester.tap(find.text("下一页"));
        await tester.pumpAndSettle();
        expect(find.text("72 个重复图像组"), findsOneWidget);
        expect(cards, findsOneWidget);
        await tester.tap(find.text("上一页"));
        await tester.pumpAndSettle();
        await tester.tap(find.text("查看重复位置").first);
        await tester.pumpAndSettle();
        final title = find.byKey(const Key("duplicate-review-title"));
        expect(tester.widget<Text>(title).data, "one.png");
        expect(find.text("合并重复图片时，优先显示你选择的这一份。"), findsNothing);
        expect(find.text("72 个重复图像组"), findsNothing);
        expect(find.text("首选副本"), findsNothing);
        expect(find.text("本地可用"), findsNothing);
        final grid = find.byType(GridView);
        expect(tester.getTopLeft(grid).dx, tester.getTopLeft(title).dx);
        expect(
          tester
              .getTopLeft(find.byKey(const Key("duplicate-review-actions")))
              .dx,
          tester.getTopLeft(title).dx,
        );
        for (final tile in tester.widgetList<ListTile>(find.byType(ListTile))) {
          expect(tile.leading, isNull);
          expect(tile.trailing, isNotNull);
        }
        if (width > 1000) {
          expect(
            tester.getTopLeft(find.text("忽略此组")).dy,
            tester.getTopLeft(find.text("暂停查重")).dy,
          );
        }
        service.pendingMembers = Completer();
        await tester.tap(find.text("刷新结果"));
        await tester.pump();
        final progress = find.byKey(const Key("library-top-loading"));
        expect(tester.getSize(progress).height, 2);
        expect(tester.widget<LinearProgressIndicator>(progress).minHeight, 2);
        service.pendingMembers!.complete(service.members);
        await tester.pumpAndSettle();
        expect(progress, findsNothing);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }

  testWidgets(
    "grid preview demand covers visible columns and retires when hidden",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1280, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final library = ReviewFixtureLibrary();
      final active = ValueNotifier(true);
      addTearDown(active.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [libraryControllerProvider.overrideWith(() => library)],
          child: MaterialApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  width: 1200,
                  height: 280,
                  child: ValueListenableBuilder(
                    valueListenable: active,
                    builder: (_, enabled, _) => TickerMode(
                      enabled: enabled,
                      child: LibraryDuplicateReviewGrid(
                        assets: List.generate(
                          64,
                          (index) => reviewAsset("item-$index"),
                        ),
                        itemBuilder: (_, index) => Text("Image $index"),
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
      expect(
        library.demands.values.single,
        List.generate(9, (index) => "item-$index"),
      );
      final grid = tester.widget<GridView>(find.byType(GridView));
      grid.controller!.jumpTo(840);
      await tester.pumpAndSettle();
      expect(
        library.demands.values.single,
        List.generate(9, (index) => "item-${index + 15}"),
      );
      active.value = false;
      await tester.pumpAndSettle();
      expect(library.demands, isEmpty);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}

class _PagedReview extends ReviewFixtureService {
  @override
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  }) async => LibraryDuplicateReviewPage(
    totalGroups: 72,
    groups: after == null ? [entry, entry] : [entry],
    next: after == null
        ? const LibraryDuplicateGroupCursor("page-one", 1024)
        : null,
  );
}

class _NoAnalysis implements LibraryExactDuplicates {
  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) =>
      const Stream.empty();
  @override
  Future<void> cancel(String runId) async {}
  @override
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  ) async => const LibraryDuplicateGroupPage(groups: []);
  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async => const LibraryDuplicateLocationPage(locations: []);
}
