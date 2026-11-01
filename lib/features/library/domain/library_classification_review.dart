import "library_models.dart";
import "library_organization.dart";

enum LibraryClassificationReview {
  unclassified,
  suggested,
  needsReview,
  deferred,
  confirmed,
  conflict,
}

class LibraryClassificationItem {
  const LibraryClassificationItem({
    required this.assetId,
    required this.review,
    required this.canConfirm,
    this.categoryId,
  });
  final String assetId;
  final String? categoryId;
  final LibraryClassificationReview review;
  final bool canConfirm;

  String get reviewLabel => switch (review) {
    LibraryClassificationReview.unclassified => "尚未分类",
    LibraryClassificationReview.confirmed => "已确认",
    LibraryClassificationReview.conflict => "分类冲突",
    _ => "待确认",
  };
}

class LibraryClassificationScope {
  LibraryClassificationScope({
    required this.query,
    required this.catalogRevision,
    required this.allMatching,
    required Iterable<String> assetIds,
    this.targets = const [],
  }) : assetIds = List.unmodifiable(assetIds);
  final LibraryGalleryQuery query;
  final BigInt catalogRevision;
  final bool allMatching;
  final List<String> assetIds;
  final List<LibraryOrganizationTarget> targets;
}

class LibraryClassificationSummary {
  const LibraryClassificationSummary({
    required this.groups,
    required this.confirmable,
    required this.confirmed,
    this.commonCategoryId,
    this.singleItem,
  });
  final int groups;
  final int confirmable;
  final int confirmed;
  final String? commonCategoryId;
  final LibraryClassificationItem? singleItem;
}

sealed class LibraryClassificationCommand {
  const LibraryClassificationCommand();
}

class LibraryConfirmClassification extends LibraryClassificationCommand {
  const LibraryConfirmClassification();
}

class LibraryCorrectClassification extends LibraryClassificationCommand {
  const LibraryCorrectClassification(this.categoryId);
  final String categoryId;
}

class LibraryDeferClassification extends LibraryClassificationCommand {
  const LibraryDeferClassification();
}

class LibraryClassificationReceipt {
  const LibraryClassificationReceipt(this.operationId, this.changedGroups);
  final String operationId;
  final int changedGroups;
}

class LibraryClassificationFailure implements Exception {
  const LibraryClassificationFailure(this.code, this.message);
  final String code;
  final String message;
}
