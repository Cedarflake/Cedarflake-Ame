import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_annotations.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../support/library_classification_review_fixture.dart";

void main() {
  late _Annotations service;
  late _Library library;
  late ProviderContainer container;

  setUp(() {
    service = _Annotations();
    library = _Library();
    container = ProviderContainer(
      overrides: [
        libraryControllerProvider.overrideWith(() => library),
        libraryClassificationReviewServiceProvider.overrideWithValue(service),
      ],
    );
    container.listen(libraryClassificationAnnotationsProvider, (_, _) {});
  });
  tearDown(() => container.dispose());

  test("only the smart album window is read in bounded batches", () async {
    expect(
      await container.read(libraryClassificationAnnotationsProvider.future),
      isEmpty,
    );
    expect(service.requests, isEmpty);
    library.show(List.generate(500, (index) => reviewAsset("$index")));
    await container.read(libraryClassificationAnnotationsProvider.future);
    expect(service.requests.map((request) => request.targets.length), [
      256,
      244,
    ]);
    expect(
      service.requests.every((request) => request.revision == BigInt.one),
      isTrue,
    );
    expect(
      service.requests
          .expand((request) => request.targets)
          .map((target) => target.locationId)
          .toSet(),
      hasLength(500),
    );
  });

  test(
    "a replaced window cannot publish or request its remaining batch",
    () async {
      final held = Completer<Map<String, LibraryClassificationReviewReason>>();
      service.read = (targets) => targets.first.locationId == "old-0"
          ? held.future
          : Future.value({"new": LibraryClassificationReviewReason.conflict});
      await container.read(libraryClassificationAnnotationsProvider.future);
      library.show(List.generate(500, (index) => reviewAsset("old-$index")));
      await Future<void>.delayed(Duration.zero);
      expect(service.requests, hasLength(1));
      library.show([reviewAsset("new")]);
      expect(
        await container.read(libraryClassificationAnnotationsProvider.future),
        {"new": LibraryClassificationReviewReason.conflict},
      );
      held.complete({"old-0": LibraryClassificationReviewReason.uncertain});
      await Future<void>.delayed(Duration.zero);
      expect(service.requests, hasLength(2));
      expect(
        container
            .read(libraryClassificationAnnotationsProvider)
            .requireValue
            .keys,
        ["new"],
      );
    },
  );

  test(
    "failed batches expose no partial annotations and can be reloaded",
    () async {
      service.read = (targets) async {
        if (targets.first.locationId == "256") throw StateError("read failed");
        return {"0": LibraryClassificationReviewReason.uncertain};
      };
      await container.read(libraryClassificationAnnotationsProvider.future);
      library.show(List.generate(500, (index) => reviewAsset("$index")));
      await expectLater(
        container.read(libraryClassificationAnnotationsProvider.future),
        throwsStateError,
      );
      expect(
        container.read(libraryClassificationAnnotationsProvider).hasError,
        isTrue,
      );
      service.read = (_) async => {};
      container.invalidate(libraryClassificationAnnotationsProvider);
      expect(
        await container.read(libraryClassificationAnnotationsProvider.future),
        isEmpty,
      );
    },
  );
}

class _Annotations extends ClassificationReviewFixture {
  final requests =
      <({BigInt revision, List<LibraryOrganizationTarget> targets})>[];
  Future<Map<String, LibraryClassificationReviewReason>> Function(
    List<LibraryOrganizationTarget>,
  )?
  read;

  @override
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  ) {
    requests.add((revision: revision, targets: targets));
    return read?.call(targets) ?? Future.value({});
  }
}

class _Library extends ReviewFixtureLibrary {
  void show(List<LibraryAsset> assets) {
    state = LibraryState(
      catalogRevision: BigInt.one,
      query: const LibraryGalleryQuery(
        collection: LibraryGalleryCollection.smartAlbum(),
      ),
      assets: assets,
    );
  }
}
