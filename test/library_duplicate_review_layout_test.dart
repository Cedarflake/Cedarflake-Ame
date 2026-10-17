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
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_message.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  for (final width in [400.0, 800.0, 1200.0, 1280.0]) {
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
        final groupDelegate =
            tester.widget<GridView>(find.byType(GridView)).gridDelegate
                as SliverGridDelegateWithFixedCrossAxisCount;
        if (width > 1000) {
          expect(groupDelegate.crossAxisCount, greaterThan(1));
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
        final category = find.byKey(const Key("duplicate-review-category"));
        final actions = find.byKey(const Key("duplicate-review-actions"));
        final heading = find.byKey(const Key("duplicate-review-title"));
        final groupCount = find.byKey(const Key("duplicate-review-count"));
        final results = tester.getRect(find.byType(GridView));
        expect(tester.getCenter(category).dx, results.center.dx);
        expect(tester.getTopRight(actions).dx, results.right);
        if (width > 1000 && scale == 1) {
          expect(tester.getCenter(category).dy, tester.getCenter(actions).dy);
          expect(
            tester.getTopLeft(category).dx,
            greaterThan(tester.getTopRight(heading).dx),
          );
          expect(
            tester.getTopLeft(actions).dx,
            greaterThan(tester.getTopRight(category).dx),
          );
        }
        if (width == 400) {
          expect(
            tester.getTopLeft(category).dy,
            greaterThan(tester.getBottomLeft(groupCount).dy),
          );
          expect(
            tester.getTopLeft(heading).dx,
            tester.getTopLeft(cards.first).dx,
          );
        }
        final categoryTop = tester.getTopLeft(category).dy;
        service.pendingGroups = Completer();
        await tester.tap(find.text("已忽略"));
        await tester.pump();
        expect(tester.getTopLeft(category).dy, categoryTop);
        service.pendingGroups!.complete(
          const LibraryDuplicateReviewPage(groups: [], totalGroups: 0),
        );
        await tester.pumpAndSettle();
        expect(find.text("0 个重复图像组"), findsOneWidget);
        expect(find.byType(LibraryGalleryMessage), findsOneWidget);
        expect(tester.getTopLeft(category).dy, categoryTop);
        service.pendingGroups = null;
        await tester.tap(find.text("重复组"));
        await tester.pumpAndSettle();
        expect(find.text("上一页"), findsNothing);
        expect(find.text("下一页"), findsNothing);
        final groupScroll = tester
            .widget<GridView>(find.byType(GridView))
            .controller!;
        groupScroll.jumpTo(groupScroll.position.maxScrollExtent);
        await tester.pumpAndSettle();
        expect(find.text("72 个重复图像组"), findsOneWidget);
        expect(service.groupCursors, contains(isNotNull));
        groupScroll.jumpTo(0);
        await tester.pumpAndSettle();
        await tester.tap(find.text("查看重复图片").first);
        await tester.pumpAndSettle();
        final title = find.byKey(const Key("duplicate-review-title"));
        expect(tester.widget<Text>(title).data, "one.png");
        expect(find.text("合并重复图片时，优先显示你选择的这一份。"), findsNothing);
        expect(find.text("72 个重复图像组"), findsNothing);
        expect(find.text("首选副本"), findsNothing);
        expect(find.text("本地可用"), findsNothing);
        final grid = find.byType(GridView);
        if (width >= 800 && scale == 1) {
          expect(tester.getCenter(title).dy, tester.getCenter(actions).dy);
          expect(
            tester.getTopLeft(actions).dx - tester.getTopRight(title).dx,
            greaterThanOrEqualTo(24),
          );
        } else {
          expect(
            tester.getTopLeft(actions).dy,
            greaterThan(tester.getBottomLeft(title).dy),
          );
        }
        expect(tester.getTopLeft(grid).dx, tester.getTopLeft(title).dx);
        expect(
          tester
              .getTopRight(find.byKey(const Key("duplicate-review-actions")))
              .dx,
          tester.getTopRight(grid).dx,
        );
        expect(find.byType(ListTile), findsNothing);
        expect(find.text("查看图片"), findsNothing);
        final memberDelegate =
            tester.widget<GridView>(grid).gridDelegate
                as SliverGridDelegateWithFixedCrossAxisCount;
        expect(memberDelegate.mainAxisExtent, greaterThanOrEqualTo(104));
        if (scale == 1) expect(memberDelegate.mainAxisExtent, lessThan(160));
        expect(memberDelegate.crossAxisCount, 1);
        final memberCard = find.byKey(const ValueKey("duplicate-card-one"));
        expect(
          tester.getCenter(find.byKey(const ValueKey("prefer-one"))).dy,
          tester.getCenter(memberCard).dy,
        );
        expect(
          tester.getCenter(find.byKey(const ValueKey("reveal-one"))).dy,
          tester.getCenter(memberCard).dy,
        );
        expect(find.text("设为首选"), findsNothing);
        expect(find.text("在文件资源管理器中打开"), findsNothing);
        if (width > 1000) {
          expect(
            tester.getTopLeft(find.text("忽略此组")).dy,
            tester.getTopLeft(find.text("暂停查重")).dy,
          );
          expect(
            tester.getCenter(find.byKey(const ValueKey("prefer-one"))).dy,
            tester.getCenter(find.byKey(const ValueKey("reveal-one"))).dy,
          );
        }
        service.pendingSave = Completer();
        await tester.tap(find.byKey(const ValueKey("prefer-one")));
        await tester.pump();
        expect(find.byType(LinearProgressIndicator), findsNothing);
        service.pendingSave!.complete(
          const LibraryDuplicateDecision(
            preferredLocationId: "one",
            preferredAssetId: "asset-one",
          ),
        );
        await tester.pumpAndSettle();
        service.pendingMembers = Completer();
        await tester.tap(find.text("刷新结果"));
        await tester.pump();
        final progress = find.byKey(const Key("library-top-loading"));
        expect(tester.getSize(progress).height, 2);
        expect(tester.widget<LinearProgressIndicator>(progress).minHeight, 2);
        expect(
          tester.getTopLeft(progress).dy,
          greaterThan(
            tester
                .getBottomLeft(
                  find.byKey(const Key("duplicate-review-actions")),
                )
                .dy,
          ),
        );
        expect(tester.getTopLeft(progress).dy, tester.getTopLeft(grid).dy - 12);
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
                        itemCount: 64,
                        assetAt: (index) => reviewAsset("item-$index"),
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
      grid.controller!.jumpTo(688);
      await tester.pumpAndSettle();
      expect(
        library.demands.values.single,
        List.generate(9, (index) => "item-${index + 12}"),
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
  final groupCursors = <LibraryDuplicateGroupCursor?>[];
  @override
  Future<LibraryDuplicateReviewPage> loadGroups(
    LibraryDuplicateGroupCursor? after, {
    required bool ignored,
  }) async {
    groupCursors.add(after);
    if (pendingGroups case final pending?) return pending.future;
    return LibraryDuplicateReviewPage(
      totalGroups: 72,
      groups: List.generate(after == null ? 64 : 8, (_) => entry),
      next: after == null
          ? const LibraryDuplicateGroupCursor("page-one", 1024)
          : null,
    );
  }
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
