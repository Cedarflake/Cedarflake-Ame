function Assert-AmeSyntheticPublicationEvidence {
    param([Parameter(Mandatory = $true)] [string]$Output)

    $records = [regex]::Matches($Output, '(?m)^AME_PUBLICATION_BENCHMARK[^\r\n]*\r?$')
    $pattern = '^AME_PUBLICATION_BENCHMARK identities=50000 staged=50000 published=50000' +
        ' publication_calls=1 commits=1 polls=(?<polls>[0-9]+) overlapping_polls=(?<overlap>[0-9]+)' +
        ' partial_observations=0 p0_pending=1 source_entries=0' +
        ' fixture_ms=(?<fixture>[0-9]+) publication_ms=(?<publication>[0-9]+)' +
        ' catalog_bytes=(?<bytes>[0-9]+)\r?$'
    if ($records.Count -ne 1) { throw "Synthetic publication evidence requires exactly one record" }
    $match = [regex]::Match($records[0].Value, $pattern)
    if (-not $match.Success) { throw "Synthetic publication evidence has an invalid exact workload schema" }
    $values = @{}
    foreach ($field in @("polls", "overlap", "fixture", "publication", "bytes")) {
        $parsed = [UInt64]0
        if (-not [UInt64]::TryParse($match.Groups[$field].Value, [ref]$parsed)) {
            throw "Synthetic publication evidence has an overflowing numeric field"
        }
        $values[$field] = $parsed
    }
    if ($values.overlap -lt 3 -or $values.overlap -gt $values.polls -or
        $values.fixture -gt 600000 -or $values.publication -gt 600000 -or $values.bytes -eq 0) {
        throw "Synthetic publication evidence does not prove bounded overlapping publication"
    }
}
