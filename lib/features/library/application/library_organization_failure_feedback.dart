import "../../../app/messages/ame_message.dart";
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

  AmeMessage get title => failure.code == "organization_undo_unavailable"
      ? request.failureTitle
      : AmeLabel.changesSaveFailed;

  AmeMessage get message => switch (failure.code) {
    "organization_undo_unavailable" => AmeLabel.organizationRecordChanged,
    "organization_history_changed" => AmeLabel.organizationHistoryChanged,
    "organization_selection_changed" => AmeLabel.organizationSelectionChanged,
    "organization_selection_expired" => AmeLabel.organizationSelectionExpired,
    _ => AmeLabel.changesUnconfirmed,
  };
}
