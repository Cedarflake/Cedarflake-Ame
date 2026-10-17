import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_locations_sheet.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  for (final reducedMotion in [false, true]) {
    testWidgets("sheet wheel motion honors reducedMotion=$reducedMotion", (
      tester,
    ) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryExactDuplicatesProvider.overrideWithValue(_Duplicates()),
          ],
          child: MaterialApp(
            home: MediaQuery(
              data: MediaQueryData(
                size: const Size(800, 600),
                disableAnimations: reducedMotion,
              ),
              child: const Scaffold(
                body: LibraryDuplicateLocationsSheet(
                  group: reviewGroup,
                  currentLocationId: "one",
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final list = find.byType(ListView);
      final scroll = tester.state<ScrollableState>(
        find.descendant(of: list, matching: find.byType(Scrollable)),
      );
      await tester.sendEventToBinding(
        PointerScrollEvent(
          kind: PointerDeviceKind.mouse,
          position: tester.getCenter(list),
          scrollDelta: const Offset(0, 120),
        ),
      );
      final immediateOffset = scroll.position.pixels;
      expect(immediateOffset, greaterThan(0));
      if (reducedMotion) {
        expect(immediateOffset, 120);
      } else {
        expect(immediateOffset, lessThan(120));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 45));
        expect(scroll.position.pixels, greaterThan(immediateOffset));
        expect(scroll.position.pixels, lessThan(120));
      }
      await tester.pumpAndSettle();
      expect(scroll.position.pixels, closeTo(120, 0.001));
      expect(tester.takeException(), isNull);
    });
  }
}

class _Duplicates extends Fake implements LibraryExactDuplicates {
  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async => LibraryDuplicateLocationPage(
    locations: List.generate(
      40,
      (index) => LibraryDuplicateLocation(
        locationId: "location-$index",
        displayPath: "C:\\fixture\\image-$index.png",
        availability: LibraryDuplicateAvailability.local,
      ),
    ),
  );
}
