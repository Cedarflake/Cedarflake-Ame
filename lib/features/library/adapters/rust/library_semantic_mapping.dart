import "../../../../src/rust/domain/organization.dart" as native;
import "../../../../src/rust/domain/semantic_results.dart" as semantic;
import "../../domain/library_semantic_search.dart";

LibrarySemanticResults mapSemanticResults(
  semantic.SemanticResultManifest value,
) => LibrarySemanticResults(
  id: value.id,
  catalogRevision: value.catalogRevision,
  featureGeneration: value.featureGeneration,
  eligibleItems: value.eligibleItems.toInt(),
  indexedItems: value.indexedItems.toInt(),
  storageFull: value.storageFull,
  items: [
    for (final item in value.items)
      (
        assetId: item.assetId,
        locationId: item.locationId,
        sourceGeneration: item.sourceGeneration,
      ),
  ],
);

semantic.SemanticResultManifest mapSemanticResultsToRust(
  LibrarySemanticResults value,
) => semantic.SemanticResultManifest(
  id: value.id,
  catalogRevision: value.catalogRevision,
  featureGeneration: value.featureGeneration,
  eligibleItems: BigInt.from(value.eligibleItems),
  indexedItems: BigInt.from(value.indexedItems),
  storageFull: value.storageFull,
  items: [
    for (final item in value.items)
      native.OrganizationSelection(
        assetId: item.assetId,
        locationId: item.locationId,
        sourceGeneration: item.sourceGeneration,
      ),
  ],
);
