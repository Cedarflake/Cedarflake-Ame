import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_viewport.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_review_toolbar.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_review_mode_notice.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_review_fixture.dart";

void main() {
  testWidgets(
    "review actions sit below browsing actions and leave the gallery bottom clear",
    (tester) async {
      final service = ReviewServiceFixture()
        ..saved = const LibraryReviewContinuation(
          LibraryContinuationStatus.anchorUnavailable,
        );
      final surface = ReviewSurfaceFixture();
      final mode = LibraryOrganizationMode(service, surface);
      addTearDown(mode.dispose);
      await mode.enter();
      var imageTaps = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LibraryGalleryViewport(
              header: LibraryGalleryHeaderActions(
                primary: TextButton(onPressed: () {}, child: const Text("选择")),
                secondary: LibraryReviewToolbar(mode: mode),
              ),
              isEmpty: false,
              isLoading: false,
              body: GestureDetector(
                key: const Key("photos"),
                behavior: HitTestBehavior.opaque,
                onTap: () => imageTaps++,
                child: const ColoredBox(color: Colors.blue),
              ),
            ),
          ),
        ),
      );
      final gallery = tester.getRect(find.byKey(const Key("photos")));
      final viewport = tester.getRect(find.byType(LibraryGalleryViewport));
      final queues = tester.getRect(
        find.byKey(const Key("library-review-queues")),
      );
      final toolbar = tester.getRect(
        find.byKey(const Key("library-review-toolbar")),
      );
      final browsing = tester.getRect(find.widgetWithText(TextButton, "选择"));
      expect(browsing.bottom, lessThan(toolbar.top));
      expect(browsing.right, toolbar.right);
      expect(gallery.top, toolbar.bottom);
      expect(gallery.bottom, viewport.bottom);
      expect(gallery.contains(queues.center), isFalse);
      expect(
        find.descendant(
          of: find.byKey(const Key("library-review-toolbar")),
          matching: find.byType(Text),
        ),
        findsNWidgets(2),
      );
      await tester.tapAt(Offset(gallery.left + 32, gallery.bottom - 32));
      expect(imageTaps, 1);
      await tester.tap(find.byKey(const Key("library-review-exit")));
      await tester.pumpAndSettle();
      expect(mode.isActive, isFalse);
      expect(imageTaps, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("mode failures use the shared top notice and retain retry", (
    tester,
  ) async {
    final surface = ReviewSurfaceFixture();
    final mode = LibraryOrganizationMode(ReviewServiceFixture(), surface);
    addTearDown(mode.dispose);
    await mode.enter();
    surface.publishing = () async => throw StateError("Query unavailable");
    await mode.selectQueue(LibraryOrganizationQueue.deferred);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ListenableBuilder(
            listenable: mode,
            builder: (_, _) => LibraryGalleryViewport(
              header: const SizedBox(height: 80),
              body: const SizedBox.expand(),
              isEmpty: false,
              isLoading: false,
              notice: LibraryReviewModeNotice(mode: mode),
            ),
          ),
        ),
      ),
    );
    final retry = find.byKey(const Key("library-review-retry"));
    expect(find.text(mode.problem!), findsOneWidget);
    expect(tester.getCenter(retry).dy, lessThan(160));
    surface.publishing = null;
    await tester.tap(retry);
    await tester.pumpAndSettle();
    expect(mode.problem, isNull);
    expect(mode.queue, LibraryOrganizationQueue.deferred);
    expect(retry, findsNothing);
    expect(tester.takeException(), isNull);
  });
}
