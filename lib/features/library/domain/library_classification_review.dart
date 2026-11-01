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

  bool get showsIndicator => this == analysisFailed;

  String get label => this == analysisFailed ? "分析失败" : "";

  String get description => switch (this) {
    analysisFailed => "这张图片未能完成分类。",
    _ => "",
  };
}

class LibraryClassificationItem {
  const LibraryClassificationItem({
    required this.assetId,
    required this.review,
    this.categoryId,
    this.reviewReason,
  });
  final String assetId;
  final String? categoryId;
  final LibraryClassificationReview review;
  final LibraryClassificationReviewReason? reviewReason;
}

class LibraryClassificationFailure implements Exception {
  const LibraryClassificationFailure(this.code, this.message);
  final String code;
  final String message;
}
