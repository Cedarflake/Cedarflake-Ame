import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_smart_album_menu.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_classification.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_progress_fixture.dart";
import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";
import "../support/library_review_fixture.dart";

void main() {
  Future<ProviderContainer> mount(
    WidgetTester tester,
    Widget child,
    ClassificationProgressFixture progress,
    ClassificationReviewFixture review,
  ) async {
    final container = ProviderContainer(
      overrides: [
        libraryClassificationProgressProvider.overrideWithValue(progress),
        libraryClassificationReviewServiceProvider.overrideWithValue(review),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
        libraryClassificationSettingsProvider.overrideWith(
          (ref) async => LibraryClassificationSettings(
            revision: BigInt.one,
            dispatchRevision: BigInt.one,
            mode: LibraryClassificationMode.automatic,
            runId: "run",
            installationSelected: true,
            categories: const [
              LibraryClassificationCategory(
                id: "drawing",
                label: "插画",
                descriptions: ["drawing"],
              ),
              LibraryClassificationCategory(
                id: "photo",
                label: "实拍",
                descriptions: ["photo"],
              ),
            ],
          ),
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Align(alignment: Alignment.topRight, child: child),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return container;
  }

  testWidgets(
    "ordinary viewer rebuild retains the same prepared image controls",
    (tester) async {
      final rebuild = ValueNotifier(0);
      addTearDown(rebuild.dispose);
      final progress = ClassificationProgressFixture();
      final review = ClassificationReviewFixture();
      await mount(
        tester,
        ValueListenableBuilder<int>(
          valueListenable: rebuild,
          builder: (context, value, child) => LibraryViewerClassification(
            target: organizationTarget("a"),
            query: const LibraryGalleryQuery(),
          ),
        ),
        progress,
        review,
      );
      expect(review.drafts, hasLength(1));
      rebuild.value++;
      await tester.pumpAndSettle();
      expect(review.drafts, hasLength(1));
      expect(review.drafts.single.closes, 0);
      expect(progress.scopes, isEmpty);
    },
  );

  testWidgets(
    "viewer deferral captures its real scope once and keeps the committed position on refresh",
    (tester) async {
      const query = LibraryGalleryQuery(
        collection: LibraryGalleryCollection.smartAlbum(
          categoryId: "drawing",
          review: LibraryClassificationReviewFilter.pending,
        ),
        searchText: "image",
        sortKey: LibraryGallerySortKey.fileName,
      );
      final progress = ClassificationProgressFixture();
      final review = ClassificationReviewFixture()
        ..pendingSave = Completer<LibraryClassificationReceipt>()
        ..summary = const LibraryClassificationSummary(
          groups: 1,
          confirmable: 1,
          confirmed: 0,
          commonCategoryId: "drawing",
          singleItem: LibraryClassificationItem(
            assetId: "asset-a",
            review: LibraryClassificationReview.needsReview,
            canConfirm: true,
          ),
        );
      final container = await mount(
        tester,
        LibraryViewerClassification(
          target: organizationTarget("a"),
          query: query,
        ),
        progress,
        review,
      );
      expect(progress.scopes, isEmpty);
      await tester.tap(find.byKey(const Key("library-change-classification")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("稍后确认"));
      await tester.pump();
      review.summary = const LibraryClassificationSummary(
        groups: 1,
        confirmable: 1,
        confirmed: 0,
        commonCategoryId: "drawing",
        singleItem: LibraryClassificationItem(
          assetId: "asset-a",
          review: LibraryClassificationReview.deferred,
          canConfirm: true,
        ),
      );
      review.pendingSave!.complete(
        const LibraryClassificationReceipt("defer", 1),
      );
      await tester.pumpAndSettle();
      expect(progress.scopes, [query]);
      expect(progress.targets, [organizationTarget("a")]);
      expect(
        review.drafts.first.commands.single,
        isA<LibraryDeferClassification>(),
      );
      expect(
        container.read(libraryClassificationActionsProvider).notice?.actionId,
        "classification.undo",
      );
      await tester.tap(find.byKey(const Key("library-change-classification")));
      await tester.pumpAndSettle();
      expect(find.text("稍后确认"), findsNothing);
      expect(find.text("实拍"), findsOneWidget);
      expect(progress.positions, hasLength(1));
      review.pendingSave = null;
      await tester.tap(find.text("实拍"));
      await tester.pumpAndSettle();
      expect(progress.positions, hasLength(1));
      expect(review.drafts[1].reviewPositions.single.sequence, BigInt.from(3));
      expect(
        review.drafts[1].commands.single,
        isA<LibraryCorrectClassification>(),
      );
    },
  );

  testWidgets(
    "more menu hides continuation before activity and resumes saved work explicitly",
    (tester) async {
      final surface = ReviewSurfaceFixture();
      final progress = ClassificationProgressFixture();
      final container = await mount(
        tester,
        LibrarySmartAlbumMenu(onSelectAll: () {}, surface: surface),
        progress,
        ClassificationReviewFixture(),
      );
      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      expect(find.text("继续确认分类"), findsNothing);
      await tester.tapAt(const Offset(20, 400));
      await tester.pumpAndSettle();
      final target = LibraryOrganizationTarget.fromAsset(
        surface.library.assets.first,
      );
      progress.saved = LibraryClassificationContinuation(
        LibraryClassificationContinuationStatus.ready,
        query: const LibraryGalleryQuery(
          collection: LibraryGalleryCollection.smartAlbum(),
        ),
        target: target,
      );
      container.invalidate(libraryClassificationContinuationProvider);
      await tester.pumpAndSettle();
      expect(surface.opened, isEmpty);
      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("继续确认分类"));
      await tester.pumpAndSettle();
      expect(surface.opened.single.assetId, target.assetId);
      expect(surface.restored, isNull);
      expect(tester.takeException(), isNull);
    },
  );
}
