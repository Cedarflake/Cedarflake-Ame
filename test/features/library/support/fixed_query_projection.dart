import "package:cedarflake_ame/features/library/application/library_query_projection.dart";
import "package:cedarflake_ame/features/library/application/library_query_update.dart";
import "package:cedarflake_ame/features/library/domain/library_query_snapshot.dart";

class FixedQueryProjection implements LibraryQueryProjection {
  const FixedQueryProjection(this.anchor);

  final LibraryQueryAnchor anchor;

  @override
  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication,
  ) => publication(anchor);
}
