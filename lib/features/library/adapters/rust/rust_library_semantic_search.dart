import "../../../../src/rust/api/semantic_search.dart" as native;
import "../../../../src/rust/domain.dart" as model;
import "../../../../src/rust/domain/organization.dart" as organization;
import "../../../../src/rust/domain/semantic_results.dart" as semantic;
import "../../application/ports/library_semantic_search.dart";
import "../../domain/library_models.dart";
import "../../domain/library_semantic_search.dart";
import "library_gallery_query_mapping.dart";
import "library_semantic_mapping.dart";

class RustLibrarySemanticSearch implements LibrarySemanticSearch {
  const RustLibrarySemanticSearch();

  @override
  Future<LibrarySemanticSearchRequest> prepare(
    LibraryGalleryQuery scope,
    LibrarySemanticInput input,
  ) => _invoke(() async {
    final request = await native.prepareSemanticSearch(
      scope: mapLibraryGalleryQueryToRust(scope),
      input: switch (input) {
        LibrarySemanticDescription(:final text) =>
          semantic.SemanticSearchInput.text(description: text),
        LibrarySemanticReference(:final source) =>
          semantic.SemanticSearchInput.image(
            source: organization.OrganizationSelection(
              assetId: source.assetId,
              locationId: source.locationId,
              sourceGeneration: source.sourceGeneration,
            ),
          ),
      },
    );
    return _Request(request);
  });

  @override
  Future<LibrarySemanticIndexStatus?> indexStatus() => _invoke(() async {
    final status = await native.semanticIndexStatus();
    return status == null
        ? null
        : LibrarySemanticIndexStatus(
            indexing: status.indexing,
            blocked: status.blocked,
            catalogRevision: status.catalogRevision,
            featureGeneration: status.featureGeneration,
            budgetReached: status.budgetReached,
            issueCode: status.issueCode,
          );
  });

  @override
  Future<void> retryIndex() => _invoke(native.retrySemanticIndex);
}

class _Request implements LibrarySemanticSearchRequest {
  _Request(this.request);
  final native.SemanticSearchRequest request;
  @override
  Future<LibrarySemanticResults> run() =>
      _invoke(() async => mapSemanticResults(await request.run()));
  @override
  void cancel() => request.cancel();
  @override
  void dispose() => request.dispose();
}

Future<T> _invoke<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on model.ScanError catch (error) {
    throw LibrarySemanticFailure(error.code);
  }
}
