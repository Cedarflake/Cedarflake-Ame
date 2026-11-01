import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/library_classification_continuation.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_progress_fixture.dart";
import "../support/library_review_fixture.dart";

void main() {
  const query = LibraryGalleryQuery(
    collection: LibraryGalleryCollection.smartAlbum(
      categoryId: "drawing",
      review: LibraryClassificationReviewFilter.pending,
    ),
    sortKey: LibraryGallerySortKey.fileName,
    searchText: "image",
  );

  test(
    "failed query publication reports a retry without opening an image",
    () async {
      final surface = ReviewSurfaceFixture()..publishing = () async => false;
      final progress = ClassificationProgressFixture()
        ..saved = LibraryClassificationContinuation(
          LibraryClassificationContinuationStatus.ready,
          query: query,
          target: LibraryOrganizationTarget.fromAsset(
            surface.library.assets.first,
          ),
        );
      final navigation = LibraryClassificationContinuationNavigation(
        progress,
        surface,
      );
      expect(await navigation.resume(), "未能恢复分类确认位置，请重试");
      expect(surface.opened, isEmpty);
      navigation.dispose();
    },
  );

  test(
    "explicit continuation restores the saved query and target without restoring selections",
    () async {
      final surface = ReviewSurfaceFixture();
      final target = LibraryOrganizationTarget.fromAsset(
        surface.library.assets.first,
      );
      final progress = ClassificationProgressFixture()
        ..saved = LibraryClassificationContinuation(
          LibraryClassificationContinuationStatus.ready,
          query: query,
          target: target,
        );
      final navigation = LibraryClassificationContinuationNavigation(
        progress,
        surface,
      );
      expect(await navigation.resume(), isNull);
      expect(surface.queries, [query]);
      expect(surface.targets, [target]);
      expect(surface.opened.single.assetId, target.assetId);
      expect(surface.restored, isNull);
      navigation.dispose();
    },
  );

  test(
    "leaving the observed query retires a delayed continuation read",
    () async {
      final surface = ReviewSurfaceFixture();
      final progress = ClassificationProgressFixture()
        ..pendingContinuation = Completer<LibraryClassificationContinuation>();
      final navigation = LibraryClassificationContinuationNavigation(
        progress,
        surface,
      );
      final pending = navigation.resume();
      surface.library = surface.library.copyWith(
        query: const LibraryGalleryQuery(
          collection: LibraryGalleryCollection.favorites(),
        ),
      );
      progress.pendingContinuation!.complete(
        LibraryClassificationContinuation(
          LibraryClassificationContinuationStatus.ready,
          query: query,
          target: LibraryOrganizationTarget.fromAsset(
            surface.library.assets.first,
          ),
        ),
      );
      expect(await pending, isNull);
      expect(surface.queries, isEmpty);
      expect(surface.opened, isEmpty);
      navigation.dispose();
    },
  );

  test(
    "retiring the surface during query publication cannot open a late viewer",
    () async {
      final surface = ReviewSurfaceFixture();
      final publishing = Completer<bool>();
      surface.publishing = () => publishing.future;
      final progress = ClassificationProgressFixture()
        ..saved = LibraryClassificationContinuation(
          LibraryClassificationContinuationStatus.ready,
          query: query,
          target: LibraryOrganizationTarget.fromAsset(
            surface.library.assets.first,
          ),
        );
      final navigation = LibraryClassificationContinuationNavigation(
        progress,
        surface,
      );
      final pending = navigation.resume();
      await Future<void>.delayed(Duration.zero);
      navigation.dispose();
      publishing.complete(true);
      await pending;
      expect(surface.opened, isEmpty);
    },
  );

  test(
    "unavailable saved work reports its state without changing the current view",
    () async {
      final surface = ReviewSurfaceFixture();
      final progress = ClassificationProgressFixture()
        ..saved = const LibraryClassificationContinuation(
          LibraryClassificationContinuationStatus.scopeUnavailable,
        );
      final navigation = LibraryClassificationContinuationNavigation(
        progress,
        surface,
      );
      expect(await navigation.resume(), "上次确认的范围已不可用");
      expect(surface.queries, isEmpty);
      expect(surface.opened, isEmpty);
      navigation.dispose();
    },
  );
}
