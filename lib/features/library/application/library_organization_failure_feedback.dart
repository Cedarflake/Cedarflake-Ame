import "../domain/library_organization.dart";
import "library_organization_requests.dart";

class LibraryOrganizationFailureFeedback {
  LibraryOrganizationFailureFeedback(Object error, this.request)
    : failure = error is LibraryOrganizationFailure
          ? error
          : LibraryOrganizationFailure("unexpected_error", error.toString());

  final LibraryOrganizationFailure failure;
  final LibraryOrganizationRequest request;

  bool get canRetry => switch (failure.code) {
    "organization_undo_unavailable" ||
    "organization_history_changed" ||
    "organization_selection_expired" => false,
    _ => true,
  };

  String get title => failure.code == "organization_undo_unavailable"
      ? request.failureTitle
      : "未能保存更改";

  String get message => switch (failure.code) {
    "organization_undo_unavailable" => "图片或整理记录已更改，当前记录已保留。",
    "organization_history_changed" => "操作记录已更新，请重新打开菜单。",
    "organization_selection_changed" => "图片或整理信息已更新，请重试。",
    "organization_selection_expired" => "图片或整理信息已更新，请重新选择图片。",
    _ => "更改尚未确认保存，请重试。",
  };
}
