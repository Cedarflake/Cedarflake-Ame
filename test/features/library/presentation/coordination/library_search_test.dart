import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_query_update.dart";
import "package:cedarflake_ame/features/library/application/ports/library_semantic_search.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_semantic_search.dart";
import "package:cedarflake_ame/features/library/domain/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_search.dart";
import "package:cedarflake_ame/features/library/presentation/library_search_notifications.dart";

import "../support/library_screen_fixture.dart";

void main() {
  for (final related in [false, true]) {
    testWidgets(
      "acknowledging a failed ${related ? 'related' : 'text'} search clears only its current request",
      (tester) async {
        final container = ProviderContainer();
        final feedback = LibrarySearchNotifications(
          container.read(ameNotificationControllerProvider.notifier),
        );
        final fixture = _Fixture(onFailure: feedback.setFailure);
        try {
          fixture.query = fixture.query.copyWith(
            metadataFilter: const LibraryMetadataFilter(extensions: ["png"]),
          );
          if (related) {
            fixture.search.findRelated(
              galleryAsset(
                id: "reference",
                captureLocalTime: "2026-11-08T10:00:00",
              ),
            );
          } else {
            fixture.text.text = "fox";
          }
          await tester.pump(const Duration(milliseconds: 250));
          const failure = LibrarySemanticFailure("semantic_model_unavailable");
          fixture.service.requests.single.result.completeError(failure);
          await tester.pump();
          final old = container
              .read(ameNotificationControllerProvider)
              .current!;
          expect(fixture.search.active, isTrue);

          fixture.service.retryGate = Completer<void>();
          final retry = fixture.search.retry();
          await tester.pump();
          expect(fixture.search.loading, isTrue);
          await fixture.search.retry();
          expect(fixture.service.retries, 1);
          expect(fixture.service.requests, hasLength(1));
          fixture.service.retryGate!.complete();
          await tester.pump();
          await feedback.acknowledge(old, fixture.search);
          expect(fixture.search.active, isTrue);
          expect(fixture.search.loading, isTrue);
          fixture.service.requests.last.result.completeError(failure);
          await retry;
          await feedback.acknowledge(old, fixture.search);
          expect(fixture.search.active, isTrue);

          final current = container
              .read(ameNotificationControllerProvider)
              .current!;
          await feedback.acknowledge(current, fixture.search);
          expect(fixture.search.active, isFalse);
          expect(fixture.text.text, isEmpty);
          expect(fixture.search.referenceName, isNull);
          expect(fixture.search.error, isNull);
          expect(fixture.query.semanticResults, isNull);
          expect(fixture.query.rootId, "root-a");
          expect(fixture.query.folderRelativePath, "animals");
          expect(fixture.query.metadataFilter.extensions, ["png"]);
          expect(
            container.read(ameNotificationControllerProvider).current,
            isNull,
          );
        } finally {
          fixture.dispose();
          container.dispose();
        }
      },
    );
  }

  testWidgets(
    "superseded search cannot replace results or report an old error",
    (tester) async {
      final fixture = _Fixture();
      try {
        fixture.text.text = "红色的伞";
        await tester.pump(const Duration(milliseconds: 250));
        final old = fixture.service.requests.single;
        fixture.text.text = "蓝天";
        await tester.pump(const Duration(milliseconds: 250));
        final current = fixture.service.requests.last;
        current.result.complete(_results("current"));
        await tester.pump();
        old.result.completeError(const LibrarySemanticFailure("old_failure"));
        await tester.pump();
        expect(old.cancelled, isTrue);
        expect(fixture.query.semanticResults?.id, "current");
        expect(fixture.search.loading, isFalse);
        expect(fixture.errors.whereType<LibrarySemanticFailure>(), isEmpty);
        expect(old.disposed && current.disposed, isTrue);
      } finally {
        fixture.dispose();
      }
    },
  );

  testWidgets("late native preparation cannot cancel the current search", (
    tester,
  ) async {
    final fixture = _Fixture();
    final preparation = Completer<void>();
    fixture.service.nextPreparation = preparation;
    try {
      fixture.text.text = "红色的伞";
      await tester.pump(const Duration(milliseconds: 250));
      fixture.text.text = "蓝天";
      await tester.pump(const Duration(milliseconds: 250));
      preparation.complete();
      await tester.pump();
      final current = fixture.service.requests.singleWhere(
        (request) => !request.disposed,
      );
      current.result.complete(_results("current"));
      await tester.pump();
      expect(fixture.query.semanticResults?.id, "current");
      expect(fixture.search.loading, isFalse);
      expect(fixture.errors.whereType<LibrarySemanticFailure>(), isEmpty);
      expect(
        fixture.service.requests.every((request) => request.disposed),
        isTrue,
      );
    } finally {
      fixture.dispose();
    }
  });

  testWidgets(
    "committed input and explicit scope compose with remembered mode",
    (tester) async {
      final fixture = _Fixture();
      try {
        fixture.text.value = const TextEditingValue(
          text: "猫",
          composing: TextRange(start: 0, end: 1),
        );
        await tester.pump(const Duration(milliseconds: 400));
        expect(fixture.service.requests, isEmpty);
        fixture.text.value = fixture.text.value.copyWith(
          composing: TextRange.empty,
        );
        final submission = fixture.search.submit();
        await tester.pump();
        expect(fixture.service.scopes.single.rootId, "root-a");
        fixture.service.requests.single.result.complete(_results("scoped"));
        await submission;
        expect(fixture.preservePosition, isFalse);
        final refinement = fixture.search.changeFilters(
          fixture.query.copyWith(
            metadataFilter: const LibraryMetadataFilter(extensions: ["png"]),
          ),
        );
        await tester.pump();
        fixture.service.requests.last.result.complete(_results("filtered"));
        await refinement;
        expect(fixture.preservePosition, isTrue);
        fixture.search.selectScope(true);
        await tester.pump();
        expect(fixture.service.scopes.last.rootId, isNull);
        expect(fixture.service.scopes.last.folderRelativePath, isNull);
        fixture.service.requests.last.result.complete(_results("library"));
        await tester.pump();
        fixture.search.selectMode(LibrarySearchMode.fileName);
        await tester.pump();
        expect(fixture.savedMode, LibrarySearchMode.fileName);
        expect(fixture.query.searchText, "猫");
        expect(fixture.query.semanticResults, isNull);
        await fixture.search.clear();
        expect(fixture.query.searchText, isEmpty);
        expect(fixture.query.rootId, isNull);
      } finally {
        fixture.dispose();
      }
    },
  );
}

LibrarySemanticResults _results(String id) => LibrarySemanticResults(
  id: id,
  catalogRevision: BigInt.one,
  featureGeneration: BigInt.one,
  eligibleItems: 1,
  indexedItems: 1,
  storageFull: false,
  items: const [],
);

class _Fixture {
  _Fixture({void Function(Object?)? onFailure}) {
    search = LibrarySearch(
      text: text,
      readQuery: () => query,
      publish: (value, {required preservePosition}) async {
        query = value;
        this.preservePosition = preservePosition;
        return LibraryQueryUpdateOutcome.applied;
      },
      onFailure: (error) {
        errors.add(error);
        onFailure?.call(error);
      },
      supersedePublication: () {},
      allowContent: () => true,
      onModeChanged: (mode) => savedMode = mode,
      initialMode: LibrarySearchMode.content,
      service: service,
    );
  }
  final text = TextEditingController();
  final service = _Service();
  final errors = <Object?>[];
  LibraryGalleryQuery query = const LibraryGalleryQuery(
    rootId: "root-a",
    folderRelativePath: "animals",
  );
  LibrarySearchMode? savedMode;
  bool? preservePosition;
  late final LibrarySearch search;

  void dispose() {
    search.dispose();
    text.dispose();
  }
}

class _Service implements LibrarySemanticSearch {
  final requests = <_Request>[];
  final scopes = <LibraryGalleryQuery>[];
  Completer<void>? retryGate;
  Completer<void>? nextPreparation;
  int retries = 0;
  @override
  Future<LibrarySemanticSearchRequest> prepare(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  ) async {
    scopes.add(scope);
    final preparation = nextPreparation;
    nextPreparation = null;
    await preparation?.future;
    for (final previous in requests) {
      previous.cancel();
    }
    final request = _Request();
    requests.add(request);
    return request;
  }

  @override
  Future<LibrarySemanticIndexStatus?> indexStatus() async => null;
  @override
  Future<void> retryIndex() async {
    retries += 1;
    await retryGate?.future;
  }
}

class _Request implements LibrarySemanticSearchRequest {
  final result = Completer<LibrarySemanticResults>();
  bool cancelled = false;
  bool disposed = false;
  @override
  Future<LibrarySemanticResults> run() async {
    final value = await result.future;
    if (cancelled) throw const LibrarySemanticFailure("semantic_cancelled");
    return value;
  }

  @override
  void cancel() => cancelled = true;
  @override
  void dispose() => disposed = true;
}
