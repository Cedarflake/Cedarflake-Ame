import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review_window.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  testWidgets("member publication waits until the open viewer closes", (
    tester,
  ) async {
    final service = _PublishingMembers();
    final library = _PublishingLibrary();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryControllerProvider.overrideWith(() => library),
          libraryDuplicateReviewProvider.overrideWithValue(service),
        ],
        child: const MaterialApp(
          home: Scaffold(body: LibraryExactDuplicatesPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text("查看重复图片"));
    await tester.pumpAndSettle();

    final pending = Completer<LibraryDuplicateReviewLocations>();
    service.pendingMembers = pending;
    library.publish();
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey("duplicate-card-one")));
    await tester.pumpAndSettle();
    expect(find.byType(LibraryImageViewer), findsOneWidget);
    service.inserted = true;
    pending.complete(service.members);
    service.pendingMembers = null;
    library.publish();
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
          .asset
          .locationId,
      "two",
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
          .asset
          .locationId,
      "one",
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryImageViewer), findsNothing);
    expect(find.byKey(const ValueKey("duplicate-card-new")), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    "viewer navigation crosses a catalog batch without loading all members",
    (tester) async {
      final starts = <int>[];
      final window = LibraryDuplicateReviewWindow<LibraryAsset, int>((
        after,
      ) async {
        final start = after ?? 0;
        starts.add(start);
        return LibraryDuplicateReviewSlice(
          items: List.generate(
            64,
            (index) => reviewAsset("item-${start + index}"),
          ),
          total: 256,
          next: start < 192 ? start + 64 : null,
        );
      });
      addTearDown(window.dispose);
      await window.ensureRange(63, 64);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryDuplicateReviewViewer(
                initialAsset: window.itemAt(63)!,
                initialIndex: 63,
                totalCount: 256,
                loadAsset: (index) async {
                  await window.retryRange(index, index + 1);
                  return window.itemAt(index);
                },
                onBack: () {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();
      expect(starts, [0, 64]);
      expect(
        tester
            .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
            .asset
            .locationId,
        "item-64",
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.pumpAndSettle();
      expect(starts, [0, 64]);
      expect(
        tester
            .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
            .asset
            .locationId,
        "item-63",
      );
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("a late navigation result cannot reopen a disposed viewer", (
    tester,
  ) async {
    final next = Completer<LibraryAsset?>();
    final library = ReviewFixtureLibrary();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [libraryControllerProvider.overrideWith(() => library)],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryDuplicateReviewViewer(
              initialAsset: reviewAsset("one"),
              initialIndex: 0,
              totalCount: 2,
              loadAsset: (_) => next.future,
              onBack: () {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    await tester.pumpWidget(const SizedBox.shrink());
    next.complete(reviewAsset("two"));
    await tester.pumpAndSettle();
    expect(find.byType(LibraryImageViewer), findsNothing);
    expect(tester.takeException(), isNull);
  });
}

class _PublishingMembers extends ReviewFixtureService {
  bool inserted = false;

  @override
  LibraryDuplicateReviewLocations get members =>
      LibraryDuplicateReviewLocations(
        locations: [
          if (inserted)
            LibraryDuplicateReviewLocation(
              asset: reviewAsset("new"),
              availability: LibraryDuplicateAvailability.local,
            ),
          ...super.members.locations,
        ],
        decision: decision,
      );
}

class _PublishingLibrary extends ReviewFixtureLibrary {
  var revision = 0;

  void publish() {
    state = state.copyWith(catalogRevision: BigInt.from(++revision));
  }
}
