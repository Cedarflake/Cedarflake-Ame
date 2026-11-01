import "library_classification_review.dart";
import "library_models.dart";
import "library_organization.dart";

class LibraryClassificationPosition {
  const LibraryClassificationPosition(this.sessionId, this.sequence);
  final String sessionId;
  final BigInt sequence;
}

class LibraryClassificationProgressReceipt {
  const LibraryClassificationProgressReceipt(this.operation, this.position);
  final LibraryClassificationReceipt operation;
  final LibraryClassificationPosition position;
}

enum LibraryClassificationContinuationStatus {
  noActivity,
  ready,
  anchorUnavailable,
  scopeUnavailable,
  emptyScope,
  endOfScope;

  String get message => switch (this) {
    noActivity => "还没有保存的分类确认位置",
    ready => "继续确认分类",
    anchorUnavailable => "上次确认的图片暂不可用",
    scopeUnavailable => "上次确认的范围已不可用",
    emptyScope => "这个范围没有可确认的图片",
    endOfScope => "已到这个范围的末尾",
  };
}

class LibraryClassificationContinuation {
  const LibraryClassificationContinuation(
    this.status, {
    this.query,
    this.target,
  });
  final LibraryClassificationContinuationStatus status;
  final LibraryGalleryQuery? query;
  final LibraryOrganizationTarget? target;
  bool get hasActivity =>
      status != LibraryClassificationContinuationStatus.noActivity;
  bool get canResume => query != null && target != null;
}
