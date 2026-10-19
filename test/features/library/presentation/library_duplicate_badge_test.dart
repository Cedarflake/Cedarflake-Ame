import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_badge.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";

void main() {
  for (final update in ["catalog revision", "scroll window"]) {
    testWidgets("confirmed badge stays visible while $update reloads", (
      tester,
    ) async {
      final library = _Window();
      final service = _Annotations();
      await tester.pumpWidget(_host(library, service));
      expect(_badge("one"), findsNothing);
      service.requests.single.complete({"one": reviewGroup});
      await tester.pumpAndSettle();
      expect(_badge("one"), findsOneWidget);

      if (update == "catalog revision") {
        library.publishRevision(2);
      } else {
        library.expandWindow();
      }
      await tester.pump();
      expect(service.requests, hasLength(2));
      expect(_badge("one"), findsOneWidget);
      expect(_badge("two"), findsNothing);
      await tester.pump(const Duration(milliseconds: 500));
      expect(_badge("one"), findsOneWidget);

      service.requests.last.complete({"one": reviewGroup});
      await tester.pumpAndSettle();
      expect(_badge("one"), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    "current absence removes badge and late results cannot restore it",
    (tester) async {
      final library = _Window();
      final service = _Annotations();
      await tester.pumpWidget(_host(library, service));
      service.requests.single.complete({"one": reviewGroup});
      await tester.pumpAndSettle();

      library.publishRevision(2);
      await tester.pump();
      final older = service.requests.last;
      library.publishRevision(3);
      await tester.pump();
      expect(service.requests, hasLength(3));
      service.requests.last.complete({});
      await tester.pumpAndSettle();
      expect(_badge("one"), findsNothing);
      older.complete({"one": reviewGroup});
      await tester.pumpAndSettle();
      expect(_badge("one"), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "retained annotations stay tied to their location and display mode",
    (tester) async {
      final library = _Window();
      final service = _Annotations();
      await tester.pumpWidget(_host(library, service));
      service.requests.single.complete({"one": reviewGroup});
      await tester.pumpAndSettle();

      library.replaceLocation();
      await tester.pump();
      expect(_badge("one"), findsNothing);
      expect(_badge("two"), findsNothing);
      service.requests.last.complete({"two": reviewGroup});
      await tester.pumpAndSettle();
      expect(_badge("two"), findsOneWidget);
      library.showAll();
      await tester.pumpAndSettle();
      expect(_badge("two"), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}

Finder _badge(String id) => find.byKey(ValueKey("duplicate-badge-$id"));

Widget _host(_Window library, _Annotations service) => ProviderScope(
  overrides: [
    libraryControllerProvider.overrideWith(() => library),
    libraryGalleryDuplicatesProvider.overrideWithValue(service),
  ],
  child: MaterialApp(
    home: Scaffold(
      body: Consumer(
        builder: (context, ref, _) => Column(
          children: [
            for (final asset in ref.watch(libraryControllerProvider).assets)
              LibraryDuplicateBadge(locationId: asset.locationId),
          ],
        ),
      ),
    ),
  ),
);

class _Annotations implements LibraryGalleryDuplicates {
  final requests = <Completer<Map<String, LibraryDuplicateGroup>>>[];

  @override
  Future<Map<String, LibraryDuplicateGroup>> load(
    BigInt revision,
    List<String> locationIds,
  ) {
    final request = Completer<Map<String, LibraryDuplicateGroup>>();
    requests.add(request);
    return request.future;
  }
}

class _Window extends LibraryController {
  @override
  LibraryState build() => LibraryState(
    catalogRevision: BigInt.one,
    query: const LibraryGalleryQuery(
      duplicateMode: LibraryGalleryDuplicateMode.merged,
    ),
    assets: [reviewAsset("one")],
  );

  void publishRevision(int revision) =>
      state = state.copyWith(catalogRevision: BigInt.from(revision));

  void expandWindow() =>
      state = state.copyWith(assets: [...state.assets, reviewAsset("two")]);

  void replaceLocation() =>
      state = state.copyWith(assets: [reviewAsset("two")]);

  void showAll() => state = state.copyWith(
    query: state.query.copyWith(duplicateMode: LibraryGalleryDuplicateMode.all),
  );
}
