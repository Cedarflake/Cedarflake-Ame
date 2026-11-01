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

enum LibraryClassificationReviewReason {
  uncertain,
  uncalibrated,
  sampling,
  deferred,
  conflict,
  analysisFailed;

  bool get showsIndicator => this != sampling;

  String get label => this == analysisFailed ? "分析失败" : "待确认";

  String get description => switch (this) {
    uncertain => "几个分类的判断接近，可以确认或更改分类。",
    uncalibrated => "这个分类的准确度还未充分验证，可以确认或更改分类。",
    sampling => "模型建议，可按需抽查。",
    deferred => "已暂缓确认，可以继续确认或更改分类。",
    conflict => "重复图片有不同的人工分类，请选择一个分类。",
    analysisFailed => "这张图片未能完成分类。",
  };
}

class LibraryClassificationItem {
  const LibraryClassificationItem({
    required this.assetId,
    required this.review,
    required this.canConfirm,
    this.categoryId,
    this.reviewReason,
  });
  final String assetId;
  final String? categoryId;
  final LibraryClassificationReview review;
  final bool canConfirm;
  final LibraryClassificationReviewReason? reviewReason;

  String get reviewDescription => reviewReason?.description ?? reviewLabel;

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
