import "../../../../src/rust/domain/organization_queue.dart"
    as rust_organization;
import "../../domain/library_models.dart";

LibraryOrganizationCounts mapRustOrganizationCounts(
  rust_organization.OrganizationCounts counts,
) => LibraryOrganizationCounts(
  total: counts.total.toInt(),
  pending: counts.pending.toInt(),
  deferred: counts.deferred_.toInt(),
  organized: counts.organized.toInt(),
  ignored: counts.ignored.toInt(),
);
