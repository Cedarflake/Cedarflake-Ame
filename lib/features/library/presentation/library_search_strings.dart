import "../../../app/messages/ame_message.dart";
import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_semantic_search.dart";

abstract final class LibrarySearchStrings {
  static AmeMessage failure(Object error) => switch (error) {
    LibrarySemanticFailure(code: "semantic_model_unavailable") =>
      AmeLabel.searchNeedsModel,
    LibrarySemanticFailure(code: "semantic_query_invalid") =>
      AmeLabel.searchDescriptionTooLong,
    LibrarySemanticFailure(code: "semantic_index_blocked") =>
      AmeLabel.searchPartialAnalysis,
    LibrarySemanticFailure(code: "semantic_catalog_changed") =>
      AmeLabel.searchLibraryChanged,
    LibrarySemanticFailure(code: "semantic_source_changed") =>
      AmeLabel.searchPhotoChanged,
    LibrarySemanticFailure(code: "semantic_session_unavailable") =>
      AmeLabel.searchAnalysisUnavailable,
    LibrarySemanticFailure(code: "semantic_storage_invalid") =>
      AmeLabel.searchIndexDamaged,
    LibrarySemanticFailure(code: "semantic_storage_failed") =>
      AmeLabel.searchStorageUnavailable,
    _ => AmeLabel.searchFailedRetained,
  };

  static String coverage(
    AppLocalizations strings,
    LibrarySemanticResults results,
  ) => strings.searchCoverage(
    results.indexedItems,
    results.eligibleItems,
    results.storageFull ? strings.analysisStorageFullSuffix : '',
  );

  static AmeMessage? notice(LibrarySemanticResults results) {
    if (results.storageFull || results.indexedItems < results.eligibleItems) {
      return AmeLabel.partialResults;
    }
    return null;
  }
}
