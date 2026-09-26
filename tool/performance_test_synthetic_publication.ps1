$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "performance_synthetic_common.ps1")

function Get-AmeSyntheticPublicationProtocolFixture {
    return "AME_PUBLICATION_BENCHMARK identities=50000 staged=50000 published=50000 publication_calls=1 commits=1 polls=25 overlapping_polls=24 partial_observations=0 p0_pending=1 source_entries=0 fixture_ms=10000 publication_ms=500 catalog_bytes=12345678"
}

$publicationName = (@(Get-AmeSyntheticCases | Where-Object Name -CEQ "publication"))[0].Test
$publicationMarker = Get-AmeSyntheticPublicationProtocolFixture
$publicationOutput = "running 1 test`ntest $publicationName ... $publicationMarker`nok`n`ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1200 filtered out; finished in 10.5s`n"
foreach ($valid in @($publicationOutput, $publicationOutput.Replace("`n", "`r`n"))) {
    Assert-AmeSyntheticResult -Output $valid -TestName $publicationName
}
$invalidEvidence = @()
foreach ($field in @("identities", "staged", "published")) {
    $invalidEvidence += $publicationMarker.Replace("${field}=50000", "${field}=49999")
}
foreach ($field in @("publication_calls", "commits")) {
    $invalidEvidence += $publicationMarker.Replace("${field}=1", "${field}=0")
    $invalidEvidence += $publicationMarker.Replace("${field}=1", "${field}=2")
}
$invalidEvidence += @(
    $publicationMarker.Replace("overlapping_polls=24", "overlapping_polls=0"),
    $publicationMarker.Replace("overlapping_polls=24", "overlapping_polls=2"),
    $publicationMarker.Replace("overlapping_polls=24", "overlapping_polls=26"),
    $publicationMarker.Replace("partial_observations=0", "partial_observations=1"),
    $publicationMarker.Replace("p0_pending=1", "p0_pending=0"),
    $publicationMarker.Replace("source_entries=0", "source_entries=1"),
    $publicationMarker.Replace("fixture_ms=10000", "fixture_ms=600001"),
    $publicationMarker.Replace("publication_ms=500", "publication_ms=600001"),
    $publicationMarker.Replace("catalog_bytes=12345678", "catalog_bytes=0"),
    $publicationMarker.Replace("catalog_bytes=12345678", "catalog_bytes=18446744073709551616"),
    $publicationMarker.Replace("catalog_bytes=12345678", "catalog_bytes=NaN"),
    $publicationMarker.Replace("catalog_bytes=12345678", "catalog_bytes=-1"),
    $publicationMarker.Replace("catalog_bytes=12345678", "catalog_bytes=1.5"),
    $publicationMarker.Replace("polls=25 ", ""),
    $publicationMarker.Replace("polls=25 ", "polls=25 polls=25 "),
    "$publicationMarker extra=1",
    "$publicationMarker`n$publicationMarker"
)
foreach ($invalid in $invalidEvidence) {
    $refused = $false
    try { Assert-AmeSyntheticResult -Output $publicationOutput.Replace($publicationMarker, $invalid) -TestName $publicationName }
    catch {
        if ($_.Exception.Message -notmatch '^Synthetic (publication|execution)') { throw }
        $refused = $true
    }
    if (-not $refused) { throw "Synthetic publication guard accepted incomplete or invalid evidence" }
}
Write-Host "AME_SYNTHETIC_PUBLICATION_PROTOCOL identities=50000 negative_cases=$($invalidEvidence.Count) overlap_and_commit=passed"
