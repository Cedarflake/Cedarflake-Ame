import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/application/library_review.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_workbench_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_batch_fixture.dart";
import "../support/library_organization_fixture.dart";
import "../support/library_review_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "organization follows Library sources and is absent from collections",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final state = populatedScreenState(totalItems: 12);
      final review = ReviewServiceFixture();
      final organization = OrganizationServiceFixture()
        ..albumItems = [const LibraryOrganizationAlbum("a", "Travel")];
      final catalog = _Catalog(
        LibrarySnapshot(
          catalogPath: state.catalogPath!,
          revision: state.catalogRevision!,
          queryId: state.queryId,
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(organization),
          libraryReviewProvider.overrideWithValue(review),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-sidebar-favorites")), findsNothing);
      await tester.tap(find.byKey(const ValueKey("source-title-root-1")));
      await tester.pumpAndSettle();
      await _more(tester, "整理模式");
      expect(catalog.loadQueries.last.rootId, "root-1");
      expect(
        catalog.loadQueries.last.organizationQueue,
        LibraryOrganizationQueue.pending,
      );
      await tester.tap(find.byKey(const Key("library-sidebar-library")));
      await tester.pumpAndSettle();
      expect(catalog.loadQueries.last.rootId, isNull);
      expect(catalog.loadQueries.last.folderRelativePath, isNull);
      expect(
        catalog.loadQueries.last.organizationQueue,
        LibraryOrganizationQueue.pending,
      );
      await tester.tap(find.byKey(const Key("library-select-button")));
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey("select-location-1")));
      await tester.pump();
      expect(find.text("已选择 1 个项目"), findsOneWidget);
      for (final destination in [
        "workbench-duplicates",
        "library-sidebar-settings",
      ]) {
        await tester.tap(find.byKey(Key(destination)));
        await tester.pumpAndSettle();
        expect(find.byKey(const Key("library-review-dock")), findsNothing);
        await tester.tap(find.byKey(const Key("workbench-gallery")));
        await tester.pumpAndSettle();
        expect(find.byKey(const Key("library-review-dock")), findsOneWidget);
        expect(find.text("已选择 1 个项目"), findsOneWidget);
        expect(
          container.read(libraryControllerProvider).query.organizationQueue,
          LibraryOrganizationQueue.pending,
        );
      }
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-review-dock")), findsNothing);
      for (final key in const [
        "library-sidebar-favorites",
        "library-sidebar-album-a",
      ]) {
        await tester.tap(find.byKey(ValueKey(key)));
        await tester.pumpAndSettle();
        expect(catalog.loadQueries.last.organizationQueue, isNull);
        await tester.tap(find.byKey(const Key("library-more-menu")));
        await tester.pumpAndSettle();
        expect(find.text("全选"), findsOneWidget);
        expect(find.text("整理模式"), findsNothing);
        expect(find.text("退出整理"), findsNothing);
        await tester.binding.handlePopRoute();
        await tester.pumpAndSettle();
      }
      await tester.tap(find.byKey(const Key("workbench-gallery")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-review-dock")), findsOneWidget);
      expect(find.text("已选择 1 个项目"), findsOneWidget);
      expect(catalog.loadQueries.last.rootId, isNull);
      expect(
        catalog.loadQueries.last.organizationQueue,
        LibraryOrganizationQueue.pending,
      );
      expect(review.starts, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "gallery overflow connects queues, coherent counts, deferred removal and mode exit",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final state = populatedScreenState(totalItems: 12);
      final review = ReviewServiceFixture();
      final batches = OrganizationBatchesFixture();
      final catalog = _Catalog(
        LibrarySnapshot(
          catalogPath: state.catalogPath!,
          revision: state.catalogRevision!,
          queryId: state.queryId,
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(
            OrganizationServiceFixture(),
          ),
          libraryOrganizationBatchesProvider.overrideWithValue(batches),
          libraryReviewProvider.overrideWithValue(review),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      await _more(tester, "整理模式");
      expect(find.byKey(const Key("library-review-dock")), findsOneWidget);
      expect(find.byKey(const Key("library-review-resume")), findsNothing);
      expect(find.text("12 张图片 · 已整理 4 · 待整理 6 · 已忽略 1"), findsOneWidget);
      expect(find.text("拍摄日期未知"), findsNothing);
      expect(review.starts, isEmpty);
      await tester.tap(
        find.byKey(const ValueKey("library-review-queue-deferred")),
      );
      await tester.pumpAndSettle();
      expect(
        container.read(libraryControllerProvider).query.organizationQueue,
        LibraryOrganizationQueue.deferred,
      );
      await _more(tester, "全选");
      final remove = find.byKey(const Key("library-selection-defer"));
      expect(
        find.descendant(of: remove, matching: find.text("移除")),
        findsOneWidget,
      );
      await tester.tap(remove);
      await tester.pumpAndSettle();
      final action =
          batches.batches.single.writes.single.action
              as LibraryBatchProgressChange;
      expect(action.action, LibraryProgressAction.restore);
      expect(batches.selections.single.allMatching, isTrue);
      expect(review.checkpoints, isEmpty);
      await _more(tester, "退出整理");
      expect(container.read(libraryControllerProvider).query, state.query);
      expect(find.byKey(const Key("library-review-dock")), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      expect(find.text("选择相册"), findsOneWidget);
      container
          .read(ameNotificationControllerProvider.notifier)
          .publish(
            const AmeNotificationDraft(
              title: "相册归属冲突",
              message: "1 组图片需要选择相册，已放入稍后整理。",
              severity: AmeNotificationSeverity.warning,
              actionId: "library.open-conflicts",
              actionLabel: "查看",
            ),
          );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("notification-primary-action")));
      await tester.pumpAndSettle();
      expect(
        container.read(libraryWorkbenchControllerProvider),
        LibraryWorkspace.gallery,
      );
      expect(
        container.read(libraryControllerProvider).query.organizationQueue,
        LibraryOrganizationQueue.deferred,
      );
      expect(find.byKey(const Key("library-review-dock")), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
    },
  );
}

Future<void> _more(WidgetTester tester, String label) async {
  await tester.tap(find.byKey(const Key("library-more-menu")));
  await tester.pumpAndSettle();
  await tester.tap(find.text(label));
  await tester.pumpAndSettle();
}

class _Catalog extends RecordingQueryCatalog {
  _Catalog(super.snapshot);
  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    final result = await super.loadQuerySnapshot(
      maxItems: maxItems,
      query: query,
      anchor: anchor,
    );
    return LibraryQuerySnapshot(
      snapshot: result.snapshot,
      timeline: result.timeline,
      organizationCounts: const LibraryOrganizationCounts(
        total: 12,
        pending: 6,
        deferred: 1,
        organized: 4,
        ignored: 1,
      ),
    );
  }
}
