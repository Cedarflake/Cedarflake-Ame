import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("organization counts are hidden across catalog or query identities", () {
    const counts = LibraryOrganizationCounts(
      total: 8,
      pending: 3,
      deferred: 1,
      organized: 2,
      ignored: 2,
    );
    final state = LibraryState(
      queryId: "scope",
      catalogRevision: BigInt.two,
      organizationSummary: LibraryOrganizationCountSnapshot(
        "scope",
        BigInt.two,
        counts,
      ),
    );
    expect(state.organizationCounts, same(counts));
    expect(state.copyWith(queryId: "other").organizationCounts, isNull);
    expect(
      state.copyWith(catalogRevision: BigInt.from(3)).organizationCounts,
      isNull,
    );
    expect(
      state.copyWith(organizationSummary: null).organizationCounts,
      isNull,
    );
    expect(
      state.copyWith(isLoadingPage: true).organizationCounts,
      same(counts),
    );
  });
}
