import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_cards.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyDuplicateWorkflow(
  WidgetTester tester,
  ProviderContainer container,
) async {
  final analysis = container.read(libraryExactAnalysisControllerProvider);
  analysis.start();
  await _waitFor(tester, () => !analysis.isRunning);
  expect(analysis.hasFailure, isFalse);
  expect(analysis.progress?.phase, LibraryExactAnalysisPhase.completed);
  final exact = container.read(libraryExactDuplicatesProvider);
  final groups = await exact.loadGroups(null);
  expect(groups.groups, hasLength(1));
  final group = groups.groups.single;
  expect(group.locationCount, 2);
  expect(group.physicalFileCount, 2);
  final locations = await exact.loadLocations(group, null);
  expect(locations.locations, hasLength(2));
  expect(
    locations.locations.every(
      (value) => value.availability == LibraryDuplicateAvailability.local,
    ),
    isTrue,
  );
  await container
      .read(libraryControllerProvider.notifier)
      .refreshCurrentQuery();
  final state = container.read(libraryControllerProvider);
  final annotations = await container
      .read(libraryGalleryDuplicatesProvider)!
      .load(
        state.catalogRevision!,
        locations.locations.map((value) => value.locationId).toList(),
      );
  expect(annotations, hasLength(2));
  expect(
    annotations.values.every((value) => value.digest == group.digest),
    isTrue,
  );

  await tester.tap(find.byKey(const Key("workbench-duplicates")));
  await _waitFor(
    tester,
    () => find.byType(LibraryDuplicateGroupCard).evaluate().isNotEmpty,
  );
  await tester.tap(find.byType(LibraryDuplicateGroupCard).first);
  await _waitFor(
    tester,
    () => find.byKey(const Key("duplicate-review-back")).evaluate().isNotEmpty,
  );
  final locationId = locations.locations.first.locationId;
  final preferred = find.byKey(ValueKey("prefer-$locationId"));
  await _waitFor(tester, () => preferred.evaluate().isNotEmpty);
  await tester.tap(preferred);
  final review = container.read(libraryDuplicateReviewProvider);
  final deadline = DateTime.now().add(const Duration(seconds: 10));
  while ((await review.loadLocations(
        group,
        null,
      )).decision.preferredLocationId !=
      locations.locations.first.locationId) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Preferred duplicate was not persisted");
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
  await tester.tap(find.byKey(const Key("duplicate-review-back")));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key("workbench-gallery")));
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull);
  debugPrint("AME_DUPLICATE_NATIVE_WORKFLOW_PASSED");
}

Future<void> _waitFor(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Duplicate workflow did not complete");
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
  await tester.pump();
}
