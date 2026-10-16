import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  test(
    "late list response cannot replace selected group or clear its loading owner",
    () async {
      final service = ReviewFixtureService()
        ..pendingGroups = Completer()
        ..pendingMembers = Completer();
      final controller = LibraryDuplicateReviewController(service);
      addTearDown(controller.dispose);
      final old = controller.refresh();
      final current = controller.showGroup(service.entry);
      service.pendingGroups!.complete(
        const LibraryDuplicateReviewPage(groups: []),
      );
      await old;
      expect(controller.loading, isTrue);
      expect(controller.selected?.group, reviewGroup);
      service.pendingMembers!.complete(service.members);
      await current;
      expect(controller.members.locations, hasLength(2));
      expect(controller.loading, isFalse);
    },
  );

  test(
    "failed write retains the prior choice and a successful write retires an older read",
    () async {
      final service = ReviewFixtureService();
      final controller = LibraryDuplicateReviewController(service);
      addTearDown(controller.dispose);
      await controller.showGroup(service.entry);
      service.failSave = true;
      expect(await controller.prefer(reviewAsset("one")), isFalse);
      expect(controller.saveFailed, isTrue);
      expect(controller.members.decision.preferredLocationId, isNull);
      service.failSave = false;
      service.pendingSave = Completer();
      final write = controller.prefer(reviewAsset("one"));
      service.pendingMembers = Completer();
      final oldRead = controller.refresh();
      service.pendingSave!.complete(
        const LibraryDuplicateDecision(
          preferredLocationId: "one",
          preferredAssetId: "asset-one",
        ),
      );
      expect(await write, isTrue);
      service.pendingMembers!.complete(service.members);
      await oldRead;
      expect(controller.members.decision.preferredLocationId, "one");
      expect(controller.saveFailed, isFalse);
      expect(controller.saving, isFalse);
    },
  );

  test(
    "a completed write cannot replace the choice of a newly selected group",
    () async {
      final service = ReviewFixtureService();
      final controller = LibraryDuplicateReviewController(service);
      addTearDown(controller.dispose);
      await controller.showGroup(service.entry);
      service.pendingSave = Completer();
      final write = controller.prefer(reviewAsset("one"));
      final next = service.entry;
      await controller.showGroup(next);
      service.pendingSave!.complete(
        const LibraryDuplicateDecision(
          preferredLocationId: "old",
          preferredAssetId: "old-asset",
        ),
      );
      await write;
      expect(controller.selected, same(next));
      expect(controller.members.decision.preferredLocationId, isNull);
    },
  );

  test("ignore and restore keep the preferred copy", () async {
    final service = ReviewFixtureService();
    final controller = LibraryDuplicateReviewController(service);
    addTearDown(controller.dispose);
    await controller.showGroup(service.entry);
    await controller.prefer(reviewAsset("one"));
    await controller.setIgnored(true);
    await controller.showGroups();
    expect(controller.groups.groups, isEmpty);
    await controller.showGroups(ignoredGroups: true);
    expect(controller.groups.groups, hasLength(1));
    await controller.showGroup(controller.groups.groups.single);
    expect(controller.members.decision.prefers(reviewAsset("one")), isTrue);
    await controller.setIgnored(false);
    expect(controller.members.decision.preferredLocationId, "one");
  });

  test(
    "catalog publication keeps the reviewed member page and its return cursor",
    () async {
      final service = ReviewFixtureService()..nextLocationId = "boundary";
      final controller = LibraryDuplicateReviewController(service);
      addTearDown(controller.dispose);
      await controller.showGroup(service.entry);
      await controller.next();
      await controller.catalogChanged();
      expect(service.memberCursors, [null, "boundary", "boundary"]);
      expect(controller.canPrevious, isTrue);
      await controller.previous();
      expect(service.memberCursors.last, isNull);
    },
  );

  test(
    "saving keeps page navigation from retiring its pending member request",
    () async {
      final service = ReviewFixtureService()..nextLocationId = "boundary";
      final controller = LibraryDuplicateReviewController(service);
      addTearDown(controller.dispose);
      await controller.showGroup(service.entry);
      service.pendingSave = Completer();
      final save = controller.prefer(reviewAsset("one"));
      await controller.next();
      expect(service.memberCursors, [null]);
      service.pendingSave!.complete(
        const LibraryDuplicateDecision(
          preferredLocationId: "one",
          preferredAssetId: "asset-one",
        ),
      );
      expect(await save, isTrue);
      await controller.next();
      expect(service.memberCursors, [null, "boundary"]);
      service.pendingSave = Completer();
      final secondSave = controller.prefer(reviewAsset("two"));
      await controller.previous();
      expect(service.memberCursors, [null, "boundary"]);
      service.pendingSave!.complete(
        const LibraryDuplicateDecision(
          preferredLocationId: "two",
          preferredAssetId: "asset-two",
        ),
      );
      await secondSave;
      await controller.previous();
      expect(service.memberCursors, [null, "boundary", null]);
    },
  );

  testWidgets(
    "review saves a preferred copy and moves ignored groups between the two lists",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1100, 850));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final service = ReviewFixtureService();
      final analysis = LibraryExactAnalysisController(_NoAnalysis());
      final library = ReviewFixtureLibrary();
      final container = ProviderContainer(
        overrides: [
          libraryControllerProvider.overrideWith(() => library),
          libraryDuplicateReviewProvider.overrideWithValue(service),
          libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(analysis.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: const MaterialApp(
            home: Scaffold(body: LibraryExactDuplicatesPage()),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text("查看重复位置"));
      await tester.pumpAndSettle();
      expect(find.text("one.png"), findsOneWidget);
      expect(find.text("two.png"), findsOneWidget);
      expect(
        tester
            .widget<TextButton>(find.byKey(const ValueKey("prefer-two")))
            .onPressed,
        isNull,
      );
      await tester.tap(find.byKey(const ValueKey("prefer-one")));
      await tester.pumpAndSettle();
      expect(service.decision.preferredLocationId, "one");
      expect(library.refreshes, 1);
      expect(find.text("取消首选"), findsOneWidget);
      await tester.tap(find.text("查看图片").first);
      await tester.pumpAndSettle();
      expect(find.byType(LibraryImageViewer), findsOneWidget);
      expect(find.byType(LibraryExactDuplicatesPage), findsNothing);
      expect(library.demands, isEmpty);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
            .asset
            .locationId,
        "two",
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(find.byType(LibraryExactDuplicatesPage), findsOneWidget);
      expect(find.text("取消首选"), findsOneWidget);
      await tester.tap(find.text("忽略此组"));
      await tester.pumpAndSettle();
      expect(service.decision.ignored, isTrue);
      expect(find.text("查看重复位置"), findsNothing);
      await tester.tap(find.text("已忽略"));
      await tester.pumpAndSettle();
      await tester.tap(find.text("查看重复位置"));
      await tester.pumpAndSettle();
      expect(find.text("首选副本"), findsOneWidget);
      await tester.tap(find.text("恢复此组"));
      await tester.pumpAndSettle();
      expect(service.decision.ignored, isFalse);
      expect(service.decision.preferredLocationId, "one");
      await tester.pumpWidget(const SizedBox.shrink());
      expect(library.demands, isEmpty);
      expect(tester.takeException(), isNull);
    },
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
