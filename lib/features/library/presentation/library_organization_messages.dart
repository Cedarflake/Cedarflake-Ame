import "../../../app/messages/ame_message.dart";

AmeMessage libraryOrganizationErrorMessage(String code) => switch (code) {
  "organization_album_name_invalid" => AmeLabel.albumNameInvalid,
  "organization_album_name_exists" => AmeLabel.albumNameTaken,
  "organization_album_unavailable" => AmeLabel.albumMissing,
  "organization_selection_changed" => AmeLabel.organizationDataChanged,
  "organization_review_changed" => AmeLabel.organizationPositionChanged,
  "organization_selection_expired" => AmeLabel.organizationReselectPhotos,
  "organization_selection_empty" => AmeLabel.noActionablePhotos,
  "organization_selection_busy" => AmeLabel.previousOperationRunning,
  "organization_conflict_unresolved" => AmeLabel.chooseConflictAlbum,
  "organization_evidence_pending" => AmeLabel.photoInformationUpdating,
  _ => AmeLabel.saveResultUnknown,
};
