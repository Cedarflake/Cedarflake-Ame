enum LibraryOrganizationQueue { pending, deferred, ignored }

class LibraryOrganizationCountSnapshot {
  const LibraryOrganizationCountSnapshot(
    this.queryId,
    this.revision,
    this.counts,
  );
  final String queryId;
  final BigInt revision;
  final LibraryOrganizationCounts counts;

  LibraryOrganizationCounts? forQuery(
    String currentQueryId,
    BigInt? currentRevision,
  ) => queryId == currentQueryId && revision == currentRevision ? counts : null;
}

class LibraryOrganizationCounts {
  const LibraryOrganizationCounts({
    required this.total,
    required this.pending,
    required this.deferred,
    required this.organized,
    required this.ignored,
  });

  final int total;
  final int pending;
  final int deferred;
  final int organized;
  final int ignored;

  int countFor(LibraryOrganizationQueue queue) => switch (queue) {
    LibraryOrganizationQueue.pending => pending,
    LibraryOrganizationQueue.deferred => deferred,
    LibraryOrganizationQueue.ignored => ignored,
  };
}
