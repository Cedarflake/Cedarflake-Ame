import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
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
    bool organizing = true,
    double scale = 1,
  }) async {
    if (organizing) await mode.enter();
    if (viewer && organizing) mode.open(surface.library.assets.single);
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
                          Align(
                            alignment: Alignment.bottomCenter,
                            child: LibraryViewerOrganizationControls(
                              mode: mode,
                              target: LibraryOrganizationTarget.fromAsset(
                                surface.library.assets.single,
                              ),
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
    "ordinary viewer omits the organization menu and progress actions",
    (tester) async {
      await host(tester, viewer: true, organizing: false);
      expect(find.byKey(const Key("viewer-review-defer")), findsNothing);
      expect(
        find.byKey(const Key("library-review-actions-menu")),
        findsNothing,
      );
      expect(find.text("忽略此图"), findsNothing);
      expect(find.text("撤销操作"), findsNothing);
      expect(find.text("重做操作"), findsNothing);
      expect(review.writes, isEmpty);
    },
  );

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
      expect(find.byKey(const Key("library-album-defer")), findsOneWidget);
      expect(find.byKey(const Key("library-album-ignore")), findsOneWidget);
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

  testWidgets("a retained album tray cannot defer a replacement review entry", (
    tester,
  ) async {
    await host(tester, viewer: true);
    final original = mode.current;
    await tester.tap(find.byKey(const Key("viewer-album-action")));
    await tester.pumpAndSettle();
    mode.open(surface.library.assets.single);
    await tester.pumpAndSettle();
    expect(mode.current, isNot(same(original)));
    await tester.tap(find.byKey(const Key("library-album-defer")));
    await tester.pumpAndSettle();
    expect(review.writes, isEmpty);
    expect(mode.current?.committed, isFalse);
  });

  for (final ignored in [false, true]) {
    testWidgets(
      "review menu ${ignored ? 'removes ignored state' : 'commits ignore'} without a favorite change",
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
        expect(find.text("恢复待整理"), findsNothing);
        expect(find.text("撤销操作"), findsOneWidget);
        expect(find.text("重做操作"), findsOneWidget);
        await tester.tap(find.text(ignored ? "移除" : "忽略此图"));
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

  for (final tray in [false, true]) {
    for (final conflict in [false, true]) {
      testWidgets(
        "deferred viewer removal uses current state, tray $tray, conflict $conflict",
        (tester) async {
          organization.inspectTarget = (_) async =>
              LibraryOrganizationInspection(
                revision: BigInt.one,
                decisions: [
                  LibraryOrganizationDecision(
                    favorite: true,
                    disposition: LibraryOrganizationDisposition.deferred,
                    hasConflict: conflict,
                  ),
                ],
              );
          review.saved = const LibraryReviewContinuation(
            LibraryContinuationStatus.emptyScope,
          );
          await host(tester, viewer: true);
          expect(mode.queue, LibraryOrganizationQueue.pending);
          if (tray) {
            await tester.tap(
              find.byKey(
                Key(conflict ? "viewer-album-conflict" : "viewer-album-action"),
              ),
            );
            await tester.pumpAndSettle();
          }
          final button = find.byKey(
            Key(tray ? "library-album-defer" : "viewer-review-defer"),
          );
          if (tray) {
            expect(
              find.descendant(of: button, matching: find.text("移除")),
              findsOneWidget,
            );
            expect(
              tester.widget<TextButton>(button).onPressed == null,
              conflict,
            );
          } else {
            expect(
              tester.widget<IconButton>(button).onPressed == null,
              conflict,
            );
            expect(find.bySemanticsLabel("移除"), findsOneWidget);
          }
          if (conflict) {
            expect(review.writes, isEmpty);
            if (tray) await tester.binding.handlePopRoute();
          } else {
            await tester.tap(button);
            await tester.pumpAndSettle();
            expect(
              (review.writes.single.action as LibraryReviewProgress).action,
              LibraryProgressAction.restore,
            );
            expect(surface.closes, 1);
          }
          expect(organization.writes, isEmpty);
          expect(organization.memberships, isEmpty);
          await tester.pump(const Duration(seconds: 6));
          await tester.pumpAndSettle();
        },
      );
    }
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
          find.byKey(const Key("library-review-queues")),
        );
        final resume = tester.getRect(
          find.byKey(const Key("library-review-resume")),
        );
        final exit = tester.getRect(
          find.byKey(const Key("library-review-exit")),
        );
        expect(exit.right, lessThan(queue.left));
        expect(queue.center.dy, closeTo(exit.center.dy, .1));
        expect(queue.height, closeTo(exit.height, .1));
        expect(queue.center.dy, closeTo(resume.center.dy, .1));
        expect(queue.height, closeTo(resume.height, .1));
        await tester.ensureVisible(find.text("稍后整理"));
        await tester.tap(find.text("稍后整理"));
        await tester.pumpAndSettle();
        expect(mode.queue, LibraryOrganizationQueue.deferred);
        expect(
          tester
              .widget<ChoiceChip>(find.widgetWithText(ChoiceChip, "稍后整理"))
              .selected,
          isTrue,
        );
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        expect(FocusManager.instance.primaryFocus?.context, isNotNull);
        final ordinary = mode.ordinaryBrowse;
        final exitButton = find.byKey(const Key("library-review-exit"));
        await tester.ensureVisible(exitButton);
        await tester.tap(exitButton);
        await tester.pumpAndSettle();
        expect(mode.isActive, isFalse);
        expect(surface.restored, same(ordinary));
        expect(surface.library.query, ordinary!.query);
        expect(review.writes, isEmpty);
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
