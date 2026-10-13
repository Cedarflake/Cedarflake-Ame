function Assert-AmeSyntheticMediaEvidence {
    param([Parameter(Mandatory = $true)] [string]$Output)

    $expected = @("jpeg", "png", "webp", "gif", "bmp", "tiff", "ico")
    $lines = @([regex]::Matches($Output, '(?m)^AME_MEDIA_FORMAT[^\r\n]*\r?$'))
    if ($lines.Count -ne $expected.Count) {
        throw "Synthetic media evidence requires exactly seven format records"
    }
    $pattern = '^AME_MEDIA_FORMAT format=(?<format>[a-z]+)' +
        ' source_width=(?<source_width>[0-9]+) source_height=(?<source_height>[0-9]+)' +
        ' source_bytes=(?<source_bytes>[0-9]+) fixture_ms=(?<fixture_ms>[0-9]+)' +
        ' cold_us=(?<cold_us>[0-9]+) warm_us=(?<warm_us>[0-9]+) bucket=(?<bucket>[0-9]+)' +
        ' preview_width=(?<preview_width>[0-9]+) preview_height=(?<preview_height>[0-9]+)' +
        ' preview_bytes=(?<preview_bytes>[0-9]+) cache_bytes=(?<cache_bytes>[0-9]+)' +
        ' source_unchanged=true cache_reused=true\r?$'
    for ($index = 0; $index -lt $expected.Count; $index++) {
        $match = [regex]::Match($lines[$index].Value, $pattern)
        if (-not $match.Success -or $match.Groups['format'].Value -cne $expected[$index]) {
            throw "Synthetic media evidence must preserve the exact format roster and field schema"
        }
        $values = @{}
        foreach ($field in @(
            "source_width", "source_height", "source_bytes", "fixture_ms", "cold_us", "warm_us",
            "bucket", "preview_width", "preview_height", "preview_bytes", "cache_bytes"
        )) {
            $parsed = [UInt64]0
            if (-not [UInt64]::TryParse($match.Groups[$field].Value, [ref]$parsed)) {
                throw "Synthetic media evidence has an overflowing numeric field"
            }
            $values[$field] = $parsed
        }
        $dimensions = if ($expected[$index] -ceq "ico") { @(256, 256, 256, 256, 256) }
            else { @(3000, 2000, 512, 512, 341) }
        $dimensionFields = @("source_width", "source_height", "bucket", "preview_width", "preview_height")
        for ($fieldIndex = 0; $fieldIndex -lt $dimensionFields.Count; $fieldIndex++) {
            if ($values[$dimensionFields[$fieldIndex]] -ne $dimensions[$fieldIndex]) {
                throw "Synthetic media evidence has an incorrect source or preview dimension"
            }
        }
        if ($values.source_bytes -eq 0 -or $values.source_bytes -gt 33554432 -or
            $values.preview_bytes -eq 0 -or $values.preview_bytes -gt 8388608 -or
            $values.cache_bytes -ne $values.preview_bytes -or
            $values.fixture_ms -gt 360000 -or $values.cold_us -gt 360000000 -or
            $values.warm_us -gt 360000000) {
            throw "Synthetic media evidence exceeds the bounded workload or cache reuse contract"
        }
    }
}
