import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_duplicates_page.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  testWidgets(
    "switching preferred copies preserves cards through catalog publication",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1100, 400));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final semantics = tester.ensureSemantics();
      try {
        final service = ReviewFixtureService()
          ..secondAvailability = LibraryDuplicateAvailability.local;
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
        for (final id in ["one", "two", "one"]) {
          final action = find.byKey(ValueKey("prefer-$id"));
          await tester.ensureVisible(action);
          await tester.pumpAndSettle();
          final card = find.byKey(ValueKey("duplicate-card-$id"));
          expect(tester.getSemantics(action).label, "设为首选");
          final element = tester.element(card);
          final scroll = tester
              .widget<GridView>(find.byType(GridView))
              .controller!;
          final offset = scroll.offset;
          service.pendingSave = Completer();
          service.pendingMembers = Completer();
          await tester.tap(action);
          await tester.pump();
          expect(find.byType(LinearProgressIndicator), findsNothing);
          service.decision = LibraryDuplicateDecision(
            preferredLocationId: id,
            preferredAssetId: "asset-$id",
          );
          service.pendingSave!.complete(service.decision);
          await tester.pumpAndSettle();
          expect(tester.widget<IconButton>(action).isSelected, isTrue);
          expect(tester.getSemantics(action).label, "取消首选");
          expect(find.byType(LinearProgressIndicator), findsNothing);
          expect(tester.element(card), same(element));
          expect(scroll.offset, offset);
          service.pendingMembers!.complete(service.members);
          await tester.pumpAndSettle();
          expect(find.byType(LinearProgressIndicator), findsNothing);
          expect(tester.element(card), same(element));
          expect(scroll.offset, offset);
        }
        expect(library.refreshes, 3);
        expect(service.memberCursors, hasLength(4));
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      } finally {
        semantics.dispose();
      }
    },
  );
}

class _PublishingLibrary extends ReviewFixtureLibrary {
  @override
  Future<bool> refreshCurrentQuery() async {
    refreshes++;
    state = state.copyWith(catalogRevision: BigInt.from(refreshes));
    return true;
  }
}
