class LibraryClassificationCounts {
  const LibraryClassificationCounts({
    required this.total,
    required this.pending,
    required this.confirmed,
  });

  final int total;
  final int pending;
  final int confirmed;
}

class LibraryClassificationCountSnapshot {
  const LibraryClassificationCountSnapshot(
    this.queryId,
    this.revision,
    this.counts,
  );

  final String queryId;
  final BigInt revision;
  final LibraryClassificationCounts counts;

  LibraryClassificationCounts? forQuery(
    String currentQueryId,
    BigInt? currentRevision,
  ) => queryId == currentQueryId && revision == currentRevision ? counts : null;
}
