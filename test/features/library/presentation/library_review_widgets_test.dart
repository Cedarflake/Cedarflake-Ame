import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_review_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_review_dock.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_organization_controls.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "../support/library_review_fixture.dart";

void main() {
  late ReviewServiceFixture review;
  late OrganizationServiceFixture organization;
  late ReviewSurfaceFixture surface;
  late LibraryOrganizationMode mode;
  late ProviderContainer container;
  setUp(() {
    review = ReviewServiceFixture();
    organization = OrganizationServiceFixture();
    surface = ReviewSurfaceFixture();
    mode = LibraryOrganizationMode(review, surface);
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(organization),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
  });
  tearDown(() {
    container.dispose();
    mode.dispose();
  });

  Future<void> host(
    WidgetTester tester, {
    bool viewer = false,
    double scale = 1,
  }) async {
    await mode.enter();
    if (viewer) mode.open(surface.library.assets.single);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(textScaler: TextScaler.linear(scale)),
            child: Scaffold(
              body: ListenableBuilder(
                listenable: mode,
                builder: (context, _) => viewer
                    ? Stack(
                        children: [
                          if (mode.current case final entry?)
                            Align(
                              alignment: Alignment.bottomCenter,
                              child: LibraryReviewActions(
                                mode: mode,
                                entry: entry,
                              ),
                            ),
                          LibraryViewerOrganizationNotice(mode: mode),
                        ],
                      )
                    : Align(
                        alignment: Alignment.bottomCenter,
                        child: LibraryReviewDock(mode: mode),
                      ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    "review album picker commits membership and position through the review service",
    (tester) async {
      organization.albumItems = [
        const LibraryOrganizationAlbum("destination", "Destination"),
      ];
      review.saved = const LibraryReviewContinuation(
        LibraryContinuationStatus.emptyScope,
      );
      await host(tester, viewer: true);
      await tester.tap(find.byKey(const Key("viewer-album-action")));
      await tester.pumpAndSettle();
      expect(review.writes, isEmpty);
      await tester.tap(find.byKey(const ValueKey("album-choice-destination")));
      await tester.pump();
      await tester.tap(find.byKey(const Key("library-album-confirm")));
      await tester.pumpAndSettle();
      final action = review.writes.single.action as LibraryReviewAlbum;
      expect(action.album?.id, "destination");
      expect(organization.memberships, isEmpty);
      expect(
        review.writes.single.position,
        same(review.checkpoints.single.position),
      );
      expect(surface.closes, 1);
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
    },
  );

  for (final ignored in [false, true]) {
    testWidgets(
      "review menu commits ${ignored ? 'restore' : 'ignore'} without a favorite change",
      (tester) async {
        organization.inspectTarget = (_) async => LibraryOrganizationInspection(
          revision: BigInt.one,
          decisions: [
            LibraryOrganizationDecision(
              favorite: true,
              disposition: ignored
                  ? LibraryOrganizationDisposition.ignored
                  : LibraryOrganizationDisposition.pending,
            ),
          ],
        );
        review.saved = const LibraryReviewContinuation(
          LibraryContinuationStatus.emptyScope,
        );
        await host(tester, viewer: true);
        await tester.tap(find.byKey(const Key("library-review-actions-menu")));
        await tester.pumpAndSettle();
        await tester.tap(find.text(ignored ? "恢复待整理" : "忽略"));
        await tester.pumpAndSettle();
        expect(
          (review.writes.single.action as LibraryReviewProgress).action,
          ignored
              ? LibraryProgressAction.restore
              : LibraryProgressAction.ignore,
        );
        expect(organization.writes, isEmpty);
        expect(surface.closes, 1);
        await tester.pump(const Duration(seconds: 6));
        await tester.pumpAndSettle();
      },
    );
  }

  for (final scale in [1.0, 2.0]) {
    testWidgets(
      "review dock at scale $scale hides first-use resume and keeps buttons aligned",
      (tester) async {
        await host(tester, scale: scale);
        expect(find.text("待整理"), findsOneWidget);
        expect(find.text("稍后整理"), findsOneWidget);
        expect(find.text("已忽略"), findsOneWidget);
        expect(find.text("继续整理"), findsNothing);
        review.saved = LibraryReviewContinuation(
          LibraryContinuationStatus.ready,
          query: surface.library.query,
          target: LibraryOrganizationTarget.fromAsset(
            surface.library.assets.single,
          ),
        );
        await mode.refreshContinuation();
        await tester.pumpAndSettle();
        final queue = tester.getRect(
          find.byType(SegmentedButton<LibraryOrganizationQueue>),
        );
        final resume = tester.getRect(
          find.byKey(const Key("library-review-resume")),
        );
        expect(queue.center.dy, closeTo(resume.center.dy, .1));
        expect(queue.height, closeTo(resume.height, .1));
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        expect(FocusManager.instance.primaryFocus?.context, isNotNull);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    "defer remains on the current image until the save receipt commits",
    (tester) async {
      final saving = Completer<LibraryReviewReceipt>();
      review.writing = (_) => saving.future;
      review.saved = const LibraryReviewContinuation(
        LibraryContinuationStatus.emptyScope,
      );
      await host(tester, viewer: true);
      await tester.tap(find.byKey(const Key("viewer-review-defer")));
      await tester.pump();
      expect(surface.closes, 0);
      expect(mode.current?.committed, isFalse);
      expect(review.writes.single.action, isA<LibraryReviewProgress>());
      saving.complete(review.receipt(review.writes.single));
      await tester.pumpAndSettle();
      expect(surface.closes, 1);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(
        container.read(ameNotificationControllerProvider).hasUnread,
        isFalse,
      );
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    "save failure reuses the top notice and retry preserves uncertain operation identity",
    (tester) async {
      review.writing = (_) async => throw StateError("Receipt unavailable");
      await host(tester, viewer: true);
      await tester.tap(find.byKey(const Key("viewer-review-defer")));
      await tester.pumpAndSettle();
      expect(surface.closes, 0);
      expect(find.text("尚未确认保存结果，请重试。"), findsOneWidget);
      expect(find.byIcon(Icons.close), findsNothing);
      final operation = review.writes.single.id;
      review.writing = null;
      review.saved = const LibraryReviewContinuation(
        LibraryContinuationStatus.emptyScope,
      );
      await tester.tap(find.text("重试"));
      await tester.pumpAndSettle();
      expect(review.writes, hasLength(2));
      expect(review.writes.last.id, operation);
      expect(organization.preparationCount, 1);
      expect(surface.closes, 1);
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
    },
  );
}
