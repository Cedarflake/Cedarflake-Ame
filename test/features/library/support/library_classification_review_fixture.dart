import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";

class ClassificationReviewFixture
    implements LibraryClassificationReviewService {
  final reads = <List<LibraryOrganizationTarget>>[];
  Completer<List<LibraryClassificationItem>>? pendingRead;
  List<LibraryClassificationItem> items = const [
    LibraryClassificationItem(
      assetId: "asset-one",
      categoryId: "drawing",
      review: LibraryClassificationReview.suggested,
    ),
  ];

  @override
  Future<List<LibraryClassificationItem>> loadItems(
    List<LibraryOrganizationTarget> targets,
  ) async {
    reads.add(List.of(targets));
    return pendingRead?.future ?? items;
  }

  @override
  Future<Map<String, LibraryClassificationReviewReason>> annotations(
    BigInt revision,
    List<LibraryOrganizationTarget> targets,
  ) async => {};
}
