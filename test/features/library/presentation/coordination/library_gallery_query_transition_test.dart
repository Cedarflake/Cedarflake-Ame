import "dart:async";
import "dart:typed_data";

import "package:cedarflake_ame/features/library/application/library_query_refresh.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_gallery_query_transition.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_filmstrip_position.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_reflow.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final background in [false, true]) {
    test(
      "merging keeps the browsed date instead of a distant representative, background=$background",
      () async {
        final fixture = _Fixture()..viewer = null;
        if (background) {
          fixture.position = fixture.position.withScrollEdge(
            LibraryGalleryScrollEdge.end,
          );
        }
        fixture.state = fixture.state.copyWith(
          query: LibraryGalleryQuery(
            duplicateMode: background
                ? LibraryGalleryDuplicateMode.merged
                : LibraryGalleryDuplicateMode.all,
          ),
          layoutManifest: _datedManifest(1, [
            for (var i = 0; i < 24; i++) ("previous-$i", "2013-09-01"),
            ("old-location", "2013-08-12"),
            ("nearby", "2013-08-12"),
            ("new-location", "2011-01-01"),
          ]),
          timeline: _mergeTimeline(1, 27),
        );
        await fixture.transition.publish((anchor) async {
          expect(anchor?.requestedLocationId, "old-location");
          fixture.state = _state(revision: 2, resolved: true).copyWith(
            query: const LibraryGalleryQuery(
              duplicateMode: LibraryGalleryDuplicateMode.merged,
            ),
            layoutManifest: _datedManifest(2, [
              ("previous", "2013-09-01"),
              ("nearby", "2013-08-12"),
              ("new-location", "2011-01-01"),
            ]),
            timeline: _mergeTimeline(2, 3),
          );
          return LibraryQueryUpdateOutcome.applied;
        });
        final retained = fixture.published.single.position!;
        expect(retained.locationId, "nearby");
        expect(retained.globalItemIndex, 1);
        expect(retained.monthKey, "2013-08");
        expect(retained.itemFraction, fixture.position.itemFraction);
        expect(retained.viewportFraction, fixture.position.viewportFraction);
        expect(retained.scrollEdge, fixture.position.scrollEdge);
      },
    );
  }
  for (final stale in [false, true]) {
    test(
      "date result retains its target before layout; stale=$stale",
      () async {
        final fixture = _Fixture()..hasLayout = false;
        final target = _asset("date-target");
        fixture.state = fixture.state.copyWith(
          assets: [_asset("before-target"), target],
          windowStartItemOffset: 500,
          timeline: LibraryTimeline(
            queryId: "query",
            revision: BigInt.one,
            totalItems: 502,
            buckets: const [
              LibraryTimeBucket(
                monthKey: "2026-01",
                itemCount: 500,
                aspectRatioSum: 500,
              ),
              LibraryTimeBucket(
                monthKey: "2025-12",
                itemCount: 2,
                aspectRatioSum: 2,
              ),
            ],
          ),
          activeTimeAnchor: LibraryTimeAnchor(
            queryId: "query",
            revision: stale ? BigInt.zero : BigInt.one,
            monthKey: "2025-12",
            itemOffset: 1,
          ),
        );
        await fixture.transition.publish((anchor) async {
          if (stale) {
            expect(anchor, isNull);
          } else {
            expect(anchor?.requestedLocationId, target.locationId);
            expect(anchor?.assetId, target.assetId);
            expect(anchor?.fallbackGlobalItemIndex, 501);
          }
          return LibraryQueryUpdateOutcome.applied;
        });
        if (!stale) {
          expect(fixture.published.single.position?.globalItemIndex, 501);
        }
      },
    );
  }

  test(
    "refresh preserves the browsed strip independently of viewer selection",
    () async {
      final fixture = _Fixture();
      final selected = _asset("viewer");
      final browsed = _asset("browsed");
      fixture.viewer = selected;
      fixture.state = fixture.state.copyWith(
        assets: [selected, browsed],
        windowStartItemOffset: 100,
      );
      fixture.filmstrip.capture(fixture.state, selected, 125, 100);
      await fixture.transition.publish((anchor) async {
        expect(anchor?.requestedLocationId, browsed.locationId);
        expect(anchor?.fallbackGlobalItemIndex, 101);
        fixture.state = fixture.state.copyWith(
          assets: [browsed],
          windowStartItemOffset: 101,
        );
        return LibraryQueryUpdateOutcome.applied;
      });
      expect(fixture.filmstrip.offsetIn(fixture.state, selected, 100), 25);
      expect(fixture.viewer, same(selected));
      fixture.filmstrip.clear();
      await fixture.transition.publish((anchor) async {
        expect(anchor?.requestedLocationId, selected.locationId);
        return LibraryQueryUpdateOutcome.applied;
      });
    },
  );

  test(
    "leaving organization restores the saved ordinary anchor and fractions",
    () async {
      final fixture = _Fixture();
      final saved = LibraryGalleryVisiblePosition(
        queryId: "saved-query",
        revision: BigInt.one,
        monthKey: null,
        locationId: "old-location",
        assetId: "saved-asset",
        globalItemIndex: 48,
        itemFraction: .6,
        viewportFraction: .7,
      );
      await fixture.transition.run((anchor) async {
        expect(anchor?.assetId, "saved-asset");
        expect(anchor?.fallbackGlobalItemIndex, 48);
        fixture.state = _state(revision: 2, resolved: true);
        return LibraryQueryUpdateOutcome.applied;
      }, retainedPosition: saved);
      final position = fixture.published.single.position!;
      expect(position.locationId, "new-location");
      expect(position.itemFraction, .6);
      expect(position.viewportFraction, .7);
    },
  );

  test(
    "folder selection resets the old position without waiting for its gesture",
    () async {
      final fixture = _Fixture();
      fixture.transition.setUserScrolling(true);
      await fixture.transition.publish((anchor) async {
        expect(anchor, isNull);
        return LibraryQueryUpdateOutcome.applied;
      }, preservePosition: false);
      expect(fixture.published.single.preservePosition, isFalse);
      expect(fixture.transition.pendingPosition, isNull);
      fixture.transition.setUserScrolling(false);
    },
  );

  test(
    "committed refresh waits for current user scrolling to finish",
    () async {
      final fixture = _Fixture();
      fixture.transition.setUserScrolling(true);
      var reads = 0;
      final result = fixture.transition.publish((_) async {
        reads += 1;
        return LibraryQueryUpdateOutcome.applied;
      });
      await Future<void>.delayed(Duration.zero);
      expect(reads, 0);
      fixture.transition.setUserScrolling(false);
      expect(await result, LibraryQueryUpdateOutcome.applied);
      expect(reads, 1);
    },
  );

  test(
    "unmount releases a gesture wait without admitting the retired projection",
    () async {
      final fixture = _Fixture();
      fixture.transition.setUserScrolling(true);
      final result = fixture.transition.publish((_) async {
        fail("Retired gallery supplied a query anchor");
      });
      fixture.transition.dispose();
      expect(await result, LibraryQueryUpdateOutcome.superseded);
      expect(fixture.published, isEmpty);
    },
  );

  test("new user query retires a gesture-delayed projection", () async {
    final fixture = _Fixture();
    fixture.transition.setUserScrolling(true);
    final old = fixture.transition.publish((_) async {
      fail("The replaced projection started a delayed read");
    });
    await fixture.transition.run(
      (_) async => LibraryQueryUpdateOutcome.applied,
    );
    fixture.transition.setUserScrolling(false);
    expect(await old, LibraryQueryUpdateOutcome.superseded);
    expect(fixture.published, hasLength(1));
  });

  test(
    "committed publication restores the resolved identity and row fraction",
    () async {
      final fixture = _Fixture();
      await fixture.transition.publish((anchor) async {
        expect(anchor?.requestedLocationId, "old-location");
        expect(anchor?.assetId, "asset");
        expect(anchor?.fallbackGlobalItemIndex, 24);
        fixture.state = _state(revision: 2, resolved: true);
        return LibraryQueryUpdateOutcome.applied;
      });
      final position = fixture.published.single.position!;
      expect(position.locationId, "new-location");
      expect(position.globalItemIndex, 524);
      expect(position.revision, BigInt.two);
      expect(position.itemFraction, 0.3);
      expect(position.viewportFraction, 0.5);
      expect(fixture.transition.pendingPosition, isNull);
      expect(fixture.reconciles, 1);
    },
  );

  test("removed anchor uses the returned fallback window", () async {
    final fixture = _Fixture();
    await fixture.transition.publish((_) async {
      fixture.state = _state(revision: 2, resolved: false);
      return LibraryQueryUpdateOutcome.applied;
    });
    final position = fixture.published.single.position!;
    expect(position.locationId, "new-location");
    expect(position.globalItemIndex, 500);
    expect(position.itemFraction, 0);
  });

  for (final hasResolution in [false, true]) {
    test(
      "unresolved wall anchor uses the fallback even when its location is loaded, resolution=$hasResolution",
      () async {
        final fixture = _Fixture();
        await fixture.transition.run((anchor) async {
          expect(anchor?.requestedLocationId, "old-location");
          fixture.state = LibraryState.fromSnapshot(
            LibrarySnapshot(
              catalogPath: "controlled.sqlite3",
              queryId: "new-query",
              revision: BigInt.two,
              roots: const [],
              assets: [_asset("window-first"), _asset("old-location")],
              queryAnchorResolution: hasResolution
                  ? const LibraryQueryAnchorResolution(
                      requestedLocationId: "old-location",
                      locationId: null,
                      ordinal: null,
                      windowStartItemOffset: 500,
                    )
                  : null,
            ),
          ).copyWith(windowStartItemOffset: 500);
          return LibraryQueryUpdateOutcome.applied;
        });
        final position = fixture.published.single.position!;
        expect(position.locationId, "window-first");
        expect(position.globalItemIndex, 500);
        expect(position.itemFraction, 0);
        expect(position.viewportFraction, 0);
      },
    );
  }

  for (final closeViewerDuringRead in [false, true]) {
    test(
      "viewer ${closeViewerDuringRead ? 'closed during' : 'open through'} refresh preserves the separate wall position",
      () async {
        final fixture = _Fixture();
        final wallAsset = _asset("old-location");
        final viewerAsset = _asset("viewer-location");
        fixture.viewer = viewerAsset;
        fixture.state = fixture.state.copyWith(
          assets: [wallAsset, viewerAsset],
          windowStartItemOffset: 24,
        );
        final read = Completer<LibraryQueryUpdateOutcome>();
        final result = fixture.transition.publish((anchor) {
          expect(anchor?.requestedLocationId, viewerAsset.locationId);
          expect(anchor?.fallbackGlobalItemIndex, 25);
          return read.future;
        });

        if (closeViewerDuringRead) {
          fixture.viewer = null;
        }
        fixture.state = _state(revision: 2, resolved: false).copyWith(
          assets: [_asset("window-first"), viewerAsset, wallAsset],
          windowStartItemOffset: 520,
          queryAnchorResolution: LibraryQueryAnchorResolution(
            requestedLocationId: viewerAsset.locationId,
            locationId: viewerAsset.locationId,
            ordinal: 521,
            windowStartItemOffset: 520,
          ),
        );
        read.complete(LibraryQueryUpdateOutcome.applied);
        expect(await result, LibraryQueryUpdateOutcome.applied);

        final position = fixture.published.single.position!;
        expect(position.locationId, wallAsset.locationId);
        expect(position.assetId, wallAsset.assetId);
        expect(position.globalItemIndex, 522);
        expect(position.revision, BigInt.two);
        expect(position.itemFraction, 0.3);
        expect(position.viewportFraction, 0.5);
        expect(fixture.transition.pendingPosition, isNull);
      },
    );
  }

  test("viewer refresh does not revive a removed wall anchor", () async {
    final fixture = _Fixture();
    final viewerAsset = _asset("viewer-location");
    fixture.viewer = viewerAsset;
    await fixture.transition.publish((_) async {
      fixture.state = _state(revision: 2, resolved: false).copyWith(
        assets: [_asset("window-first"), viewerAsset],
        queryAnchorResolution: LibraryQueryAnchorResolution(
          requestedLocationId: viewerAsset.locationId,
          locationId: viewerAsset.locationId,
          ordinal: 501,
          windowStartItemOffset: 500,
        ),
      );
      return LibraryQueryUpdateOutcome.applied;
    });
    final position = fixture.published.single.position!;
    expect(position.locationId, "window-first");
    expect(position.globalItemIndex, 500);
    expect(position.itemFraction, 0);
    expect(position.viewportFraction, 0);
  });

  for (final obsoleteFails in [false, true]) {
    test(
      "late ${obsoleteFails ? 'failure' : 'completion'} cannot clear a newer position",
      () async {
        final fixture = _Fixture();
        final first = Completer<LibraryQueryUpdateOutcome>();
        final second = Completer<LibraryQueryUpdateOutcome>();
        final old = fixture.transition.publish((_) => first.future);
        final current = fixture.transition.run((_) => second.future);
        if (obsoleteFails) {
          final expectation = expectLater(old, throwsStateError);
          first.completeError(StateError("obsolete read"));
          await expectation;
        } else {
          first.complete(LibraryQueryUpdateOutcome.applied);
          await old;
        }
        expect(fixture.transition.pendingPosition, same(fixture.position));
        expect(fixture.published, isEmpty);
        expect(fixture.failures, 0);
        second.complete(LibraryQueryUpdateOutcome.applied);
        await current;
        expect(fixture.published, hasLength(1));
      },
    );
  }

  test(
    "failed committed read releases only its presentation and preserves error",
    () async {
      final fixture = _Fixture();
      await expectLater(
        fixture.transition.publish(
          (_) => Future.error(StateError("catalog read")),
        ),
        throwsStateError,
      );
      expect(fixture.transition.pendingPosition, isNull);
      expect(fixture.failures, 1);
      expect(fixture.published, isEmpty);
    },
  );

  test(
    "disposal during viewer reconciliation cannot restore the old gallery",
    () async {
      final fixture = _Fixture();
      final reconciliation = fixture.pendingReconciliation = Completer<void>();
      final result = fixture.transition.publish(
        (_) async => LibraryQueryUpdateOutcome.applied,
      );
      await Future<void>.delayed(Duration.zero);
      expect(fixture.reconciles, 1);
      fixture.transition.dispose();
      reconciliation.complete();
      expect(await result, LibraryQueryUpdateOutcome.applied);
      expect(fixture.published, isEmpty);
      expect(fixture.transition.pendingPosition, isNull);
    },
  );

  test(
    "a replaced gallery does not cancel an already admitted application read",
    () async {
      final fixture = _Fixture();
      final completion = Completer<LibraryQueryUpdateOutcome>();
      final result = fixture.transition.publish((_) => completion.future);
      fixture.transition.dispose();
      completion.complete(LibraryQueryUpdateOutcome.applied);
      expect(await result, LibraryQueryUpdateOutcome.applied);
      expect(fixture.published, isEmpty);
      expect(fixture.reconciles, 0);
    },
  );
}

LibraryGalleryLayoutManifest _datedManifest(
  int version,
  List<(String, String)> items,
) {
  final revision = BigInt.from(version);
  final dates = items.map((item) => item.$2).toSet().toList();
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "query",
    totalItems: items.length,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "query",
      totalItems: items.length,
      startOrdinal: 0,
      locationIds: [for (final item in items) item.$1],
      aspectRatioMilli: Uint16List.fromList(List.filled(items.length, 1000)),
      dateGroupIndices: Uint16List.fromList([
        for (final item in items) dates.indexOf(item.$2),
      ]),
      dateGroups: dates,
      flags: Uint8List.fromList(
        List.filled(items.length, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}

LibraryTimeline _mergeTimeline(int version, int count) => LibraryTimeline(
  revision: BigInt.from(version),
  queryId: "query",
  totalItems: count,
  totalSourceItems: 27,
  buckets: [
    LibraryTimeBucket(
      monthKey: "2013-08",
      itemCount: count,
      aspectRatioSum: count.toDouble(),
    ),
  ],
);

class _Fixture {
  _Fixture() {
    transition = LibraryGalleryQueryTransition(
      readState: () => state,
      capturePosition: (_) => hasLayout ? position : null,
      readViewerAnchor: () => viewer,
      readViewerBrowseAnchor: filmstrip.read,
      reconcileViewer: () async {
        reconciles += 1;
        await pendingReconciliation?.future;
      },
      onPublished: published.add,
      onFailed: () => failures += 1,
    );
  }

  LibraryState state = _state(revision: 1, resolved: false);
  LibraryAsset? viewer;
  bool hasLayout = true;
  final filmstrip = LibraryFilmstripPosition();
  var position = LibraryGalleryVisiblePosition(
    queryId: "query",
    revision: BigInt.one,
    monthKey: "2013-08",
    locationId: "old-location",
    assetId: "asset",
    globalItemIndex: 24,
    itemFraction: 0.3,
    viewportFraction: 0.5,
  );
  late final LibraryGalleryQueryTransition transition;
  final published = <LibraryGalleryQueryPublication>[];
  int reconciles = 0;
  int failures = 0;
  Completer<void>? pendingReconciliation;
}

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

LibraryState _state({required int revision, required bool resolved}) =>
    LibraryState.fromSnapshot(
      LibrarySnapshot(
        catalogPath: "controlled.sqlite3",
        queryId: "query",
        revision: BigInt.from(revision),
        roots: const [],
        assets: [
          LibraryAsset(
            assetId: "asset",
            locationId: "new-location",
            rootId: "root",
            activeScanId: "scan",
            sourcePath: "generated.png",
            displayPath: "generated.png",
            relativePath: "generated.png",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            sourceRevision: null,
            sourceGeneration: BigInt.one,
            width: 32,
            height: 24,
          ),
        ],
        queryAnchorResolution: LibraryQueryAnchorResolution(
          requestedLocationId: "old-location",
          locationId: resolved ? "new-location" : null,
          ordinal: resolved ? 524 : null,
          windowStartItemOffset: 500,
        ),
      ),
    ).copyWith(windowStartItemOffset: 500);
