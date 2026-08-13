import "dart:async";
import "dart:convert";
import "dart:developer" as developer;
import "dart:io";
import "dart:ui" show FrameTiming;

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    final libraryPath =
        Platform.environment["CEDARFLAKE_AME_PROFILE_LIBRARY_PATH"];
    if (libraryPath == null || libraryPath.isEmpty) {
      throw StateError("CEDARFLAKE_AME_PROFILE_LIBRARY_PATH is required");
    }
    return RustLib.init(
      externalLibrary: ExternalLibrary.open(
        File(libraryPath).absolute.path,
        debugInfo: "Windows retained-catalog Profile library",
      ),
    );
  });

  testWidgets("profiles bounded retained-catalog gallery scrolling", (
    tester,
  ) async {
    final iterations =
        int.tryParse(
          Platform.environment["CEDARFLAKE_AME_PROFILE_ITERATIONS"] ?? "80",
        ) ??
        80;
    if (iterations < 20 || iterations > 1_000) {
      throw StateError("Profile iterations must be between 20 and 1000");
    }
    const query = LibraryGalleryQuery();
    const delegate = RustLibraryCatalog();
    final initialSnapshot = await delegate.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final timeline = await delegate.loadTimeline(query);
    if (timeline.totalItems == 0) {
      throw StateError("The retained catalog is empty");
    }
    final clock = Stopwatch()..start();
    final catalog = _ProfileCatalog(delegate, clock);
    final previewer = _ReadOnlyProfilePreviewer();
    final initialState = LibraryState.fromSnapshot(
      initialSnapshot,
      query: query,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryPreviewerProvider.overrideWithValue(previewer),
          initialLibraryStateProvider.overrideWithValue(initialState),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump(const Duration(seconds: 1));
    final wall = find.byKey(const Key("library-photo-wall"));
    expect(wall, findsOneWidget);

    final context = tester.element(wall);
    final container = ProviderScope.containerOf(context);
    var retainedDetailsMax = initialState.assets.length;
    final publicationDelayMicros = <int>[];
    final subscription = container.listen<LibraryState>(
      libraryControllerProvider,
      (previous, next) {
        if (next.assets.length > retainedDetailsMax) {
          retainedDetailsMax = next.assets.length;
        }
        if (previous != null && previous.assets.length != next.assets.length) {
          final returnedAt = catalog.takeLastPageReturnedAt();
          if (returnedAt != null) {
            publicationDelayMicros.add(clock.elapsedMicroseconds - returnedAt);
          }
        }
      },
      fireImmediately: true,
    );
    addTearDown(subscription.close);

    final frameTimings = <FrameTiming>[];
    void recordTimings(List<FrameTiming> timings) =>
        frameTimings.addAll(timings);
    WidgetsBinding.instance.addTimingsCallback(recordTimings);
    addTearDown(
      () => WidgetsBinding.instance.removeTimingsCallback(recordTimings),
    );
    final gcMonitor = await _VmGcMonitor.start();
    addTearDown(gcMonitor.close);
    final initialRss = ProcessInfo.currentRss;
    var peakRss = initialRss;

    for (var iteration = 0; iteration < iterations; iteration++) {
      final isReverse = iteration % 12 >= 9;
      await tester.fling(
        wall,
        Offset(0, isReverse ? 520 : -760),
        isReverse ? 1_200 : 1_600,
      );
      for (var frame = 0; frame < 8; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      final rss = ProcessInfo.currentRss;
      if (rss > peakRss) {
        peakRss = rss;
      }
    }
    await tester.pump(const Duration(seconds: 1));
    final finalState = container.read(libraryControllerProvider);
    final storage = await const RustStorageSettingsGateway().load();
    final finalRss = ProcessInfo.currentRss;
    if (finalRss > peakRss) {
      peakRss = finalRss;
    }
    final evidence = <String, Object?>{
      "schema": "ame-r2b-retained-profile-v1",
      "mode": "profile",
      "source_media_reads_enabled": false,
      "iterations": iterations,
      "duration_milliseconds": clock.elapsedMilliseconds,
      "catalog_total_items": timeline.totalItems,
      "retained_details_initial": initialState.assets.length,
      "retained_details_max": retainedDetailsMax,
      "retained_details_final": finalState.assets.length,
      "rss_initial_bytes": initialRss,
      "rss_peak_bytes": peakRss,
      "rss_final_bytes": finalRss,
      "gc_stream_available": gcMonitor.isAvailable,
      "gc_events": gcMonitor.eventCount,
      "frame_count": frameTimings.length,
      "build_p95_microseconds": _percentile(
        frameTimings.map((timing) => timing.buildDuration.inMicroseconds),
        0.95,
      ),
      "raster_p95_microseconds": _percentile(
        frameTimings.map((timing) => timing.rasterDuration.inMicroseconds),
        0.95,
      ),
      "ui_stalls_over_50ms": frameTimings
          .where(
            (timing) => timing.totalSpan > const Duration(milliseconds: 50),
          )
          .length,
      "catalog_page_queries": catalog.pageQueryMicros.length,
      "catalog_page_query_p95_microseconds": _percentile(
        catalog.pageQueryMicros,
        0.95,
      ),
      "page_publication_p95_microseconds": _percentile(
        publicationDelayMicros,
        0.95,
      ),
      "preview_materialize_attempts_blocked": previewer.attemptCount,
      "preview_cache_used_bytes": storage.previewUsedBytes.toString(),
      "preview_cache_budget_bytes": storage.previewBudgetBytes.toString(),
    };
    final encoded = const JsonEncoder.withIndent("  ").convert(evidence);
    stdout.writeln("AME_R2B_PROFILE $encoded");
    final evidencePath =
        Platform.environment["CEDARFLAKE_AME_PROFILE_EVIDENCE_PATH"];
    if (evidencePath != null && evidencePath.isNotEmpty) {
      final file = File(evidencePath);
      await file.parent.create(recursive: true);
      await file.writeAsString("$encoded\n", flush: true);
    }

    expect(finalState.pageErrorMessage, isNull);
    expect(retainedDetailsMax, lessThanOrEqualTo(timeline.totalItems));
    expect(finalRss, lessThan(2 * 1024 * 1024 * 1024));
  });
}

class _ProfileCatalog implements LibraryCatalog {
  _ProfileCatalog(this.delegate, this.clock);

  final LibraryCatalog delegate;
  final Stopwatch clock;
  final List<int> pageQueryMicros = [];
  int? _lastPageReturnedAt;

  int? takeLastPageReturnedAt() {
    final value = _lastPageReturnedAt;
    _lastPageReturnedAt = null;
    return value;
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    final startedAt = clock.elapsedMicroseconds;
    final snapshot = await delegate.load(
      maxItems: maxItems,
      query: query,
      after: after,
      before: before,
    );
    final returnedAt = clock.elapsedMicroseconds;
    pageQueryMicros.add(returnedAt - startedAt);
    _lastPageReturnedAt = returnedAt;
    return snapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) {
    return delegate.loadAtTime(
      maxItems: maxItems,
      query: query,
      anchor: anchor,
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) {
    return delegate.loadTimeline(query);
  }

  @override
  Future<bool> unregisterRoot(String rootId) => delegate.unregisterRoot(rootId);
}

class _ReadOnlyProfilePreviewer implements LibraryPreviewer {
  var attemptCount = 0;

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    attemptCount += 1;
    return Future.error(
      const LibraryPreviewFailure(
        code: "profile_source_reads_disabled",
        message: "Profile evidence does not read source media",
      ),
    );
  }
}

class _VmGcMonitor {
  _VmGcMonitor._({required this.isAvailable, this.socket});

  final bool isAvailable;
  final WebSocket? socket;
  StreamSubscription<dynamic>? _subscription;
  var eventCount = 0;

  static Future<_VmGcMonitor> start() async {
    try {
      final serviceInfo = await developer.Service.getInfo();
      final serverUri = serviceInfo.serverUri;
      if (serverUri == null) {
        return _VmGcMonitor._(isAvailable: false);
      }
      final monitor = _VmGcMonitor._(
        isAvailable: true,
        socket: await WebSocket.connect(
          serverUri
              .replace(
                scheme: serverUri.scheme == "https" ? "wss" : "ws",
                path: "${serverUri.path}ws",
              )
              .toString(),
        ),
      );
      monitor._subscription = monitor.socket!.listen((message) {
        final decoded = jsonDecode(message as String);
        if (decoded case {
          "method": "streamNotify",
          "params": {"streamId": "GC"},
        }) {
          monitor.eventCount += 1;
        }
      });
      monitor.socket!.add(
        jsonEncode({
          "jsonrpc": "2.0",
          "id": "ame-r2b-gc-stream",
          "method": "streamListen",
          "params": {"streamId": "GC"},
        }),
      );
      return monitor;
    } on Object {
      return _VmGcMonitor._(isAvailable: false);
    }
  }

  Future<void> close() async {
    await _subscription?.cancel();
    await socket?.close();
  }
}

int? _percentile(Iterable<int> values, double percentile) {
  final sorted = values.toList()..sort();
  if (sorted.isEmpty) {
    return null;
  }
  final index = ((sorted.length - 1) * percentile).round();
  return sorted[index];
}
