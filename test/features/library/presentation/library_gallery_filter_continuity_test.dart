import "dart:async";

import "package:cedarflake_ame/features/library/application/library_query_refresh.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_query_snapshot.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_query_transition.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final result in ["top", "bottom", "missing", "empty"]) {
    test(
      "filter expansion restores the original image after $result",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.transition.dispose);
        final original = fixture.position!;
        await fixture.filter(switch (result) {
          "empty" => [],
          "missing" => ["fallback"],
          _ => ["original"],
        });
        if (result == "top" || result == "bottom") {
          fixture.position = _position(
            "original",
            itemFraction: result == "top" ? 0 : 1,
            viewportFraction: result == "top" ? 0 : 1,
          );
        }
        await fixture.filter(["another-fallback"]);
        await fixture.filter(["before", "original", "after"]);

        expect(fixture.requests.last?.requestedLocationId, "original");
        expect(fixture.position?.locationId, "original");
        expect(fixture.position?.globalItemIndex, 1);
        expect(fixture.position?.itemFraction, original.itemFraction);
        expect(fixture.position?.viewportFraction, original.viewportFraction);
      },
    );
  }

  test(
    "browsing filtered results replaces the previous filter anchor",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.transition.dispose);
      await fixture.filter(["fallback", "browsed"]);
      fixture.transition.setUserScrolling(true);
      fixture.position = _position("browsed");
      fixture.transition.setUserScrolling(false);
      await fixture.filter(["original", "browsed"]);
      expect(fixture.requests.last?.requestedLocationId, "browsed");
      expect(fixture.position?.locationId, "browsed");
    },
  );

  test("folder selection retires a retained filter position", () async {
    final fixture = _Fixture();
    addTearDown(fixture.transition.dispose);
    await fixture.filter([]);
    await fixture.transition.run((anchor) async {
      expect(anchor, isNull);
      fixture.publish(["new-folder"], anchor);
      return LibraryQueryUpdateOutcome.applied;
    }, preservePosition: false);
    await fixture.filter(["new-folder", "original"]);
    expect(fixture.requests.last?.requestedLocationId, "new-folder");
  });

  test(
    "passive refresh preserves current results and latent filter intent",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.transition.dispose);
      await fixture.filter(["fallback"]);
      await fixture.transition.publish((anchor) async {
        expect(anchor?.requestedLocationId, "fallback");
        fixture.publish(["fallback"], anchor);
        return LibraryQueryUpdateOutcome.applied;
      });
      expect(fixture.position?.locationId, "fallback");
      await fixture.filter(["original", "fallback"]);
      expect(fixture.position?.locationId, "original");
    },
  );

  for (final fails in [false, true]) {
    test(
      "obsolete filter ${fails ? 'failure' : 'success'} cannot undo browsing",
      () async {
        final fixture = _Fixture();
        addTearDown(fixture.transition.dispose);
        final completion = Completer<LibraryQueryUpdateOutcome>();
        final obsolete = fixture.transition.runFilter((_) => completion.future);
        fixture.transition.setUserScrolling(true);
        fixture.position = _position("browsed");
        fixture.transition.setUserScrolling(false);
        await fixture.filter(["browsed"]);
        if (fails) {
          final expectation = expectLater(obsolete, throwsStateError);
          completion.completeError(StateError("obsolete filter"));
          await expectation;
        } else {
          completion.complete(LibraryQueryUpdateOutcome.applied);
          await obsolete;
        }
        await fixture.filter(["original", "browsed"]);
        expect(fixture.requests.last?.requestedLocationId, "browsed");
        expect(fixture.position?.locationId, "browsed");
      },
    );
  }

  test("timeline navigation releases an empty filter's position", () async {
    final fixture = _Fixture();
    addTearDown(fixture.transition.dispose);
    await fixture.filter([]);
    fixture.transition.clearFilterPosition();
    fixture.position = _position("navigated");
    await fixture.filter(["original", "navigated"]);
    expect(fixture.requests.last?.requestedLocationId, "navigated");
  });

  test("failed filter retry keeps the original position", () async {
    final fixture = _Fixture();
    addTearDown(fixture.transition.dispose);
    await fixture.filter(["fallback"]);
    await fixture.transition.runFilter(
      (_) async => LibraryQueryUpdateOutcome.failed,
    );
    await fixture.filter(["fallback", "original"]);
    expect(fixture.position?.locationId, "original");
  });
}

class _Fixture {
  _Fixture() {
    publish(["original"], null);
    transition = LibraryGalleryQueryTransition(
      readState: () => state,
      capturePosition: (_) => position,
      readViewerAnchor: () => null,
      reconcileViewer: () async {},
      onPublished: (publication) => position = publication.position,
      onFailed: () {},
    );
  }

  late LibraryState state;
  late final LibraryGalleryQueryTransition transition;
  LibraryGalleryVisiblePosition? position = _position("original");
  final requests = <LibraryQueryAnchor?>[];
  int revision = 0;

  Future<void> filter(List<String> locations) async {
    await transition.runFilter((anchor) async {
      requests.add(anchor);
      publish(locations, anchor);
      return LibraryQueryUpdateOutcome.applied;
    });
  }

  void publish(List<String> locations, LibraryQueryAnchor? anchor) {
    final index = locations.indexOf(anchor?.requestedLocationId ?? "");
    revision += 1;
    state = LibraryState.fromSnapshot(
      LibrarySnapshot(
        catalogPath: "controlled.sqlite3",
        revision: BigInt.from(revision),
        queryId: "filter-$revision",
        roots: const [],
        assets: locations.map(_asset).toList(),
        queryAnchorResolution: anchor == null
            ? null
            : LibraryQueryAnchorResolution(
                requestedLocationId: anchor.requestedLocationId,
                locationId: index < 0 ? null : locations[index],
                ordinal: index < 0 ? null : index,
                windowStartItemOffset: 0,
              ),
      ),
    );
  }
}

LibraryGalleryVisiblePosition _position(
  String locationId, {
  double itemFraction = 0.3,
  double viewportFraction = 0.5,
}) => LibraryGalleryVisiblePosition(
  queryId: "initial-query",
  revision: BigInt.one,
  monthKey: null,
  locationId: locationId,
  assetId: "asset-$locationId",
  globalItemIndex: 50,
  itemFraction: itemFraction,
  viewportFraction: viewportFraction,
);

LibraryAsset _asset(String locationId) => LibraryAsset(
  assetId: "asset-$locationId",
  locationId: locationId,
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "$locationId.png",
  displayPath: "$locationId.png",
  relativePath: "$locationId.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 32,
  height: 24,
);
