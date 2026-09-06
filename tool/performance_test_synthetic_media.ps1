$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "performance_synthetic_common.ps1")

function Get-AmeSyntheticMediaProtocolFixture {
    $lines = @()
    foreach ($format in @("jpeg", "png", "webp", "gif", "bmp", "tiff", "ico")) {
        $dimensions = if ($format -ceq "ico") {
            "source_width=256 source_height=256"
        } else { "source_width=3000 source_height=2000" }
        $preview = if ($format -ceq "ico") {
            "bucket=256 preview_width=256 preview_height=256"
        } else { "bucket=512 preview_width=512 preview_height=341" }
        $lines += "AME_MEDIA_FORMAT format=$format $dimensions source_bytes=1024 fixture_ms=23 cold_us=12000 warm_us=400 $preview preview_bytes=2048 cache_bytes=2048 source_unchanged=true cache_reused=true"
    }
    $lines += "AME_MEDIA_BENCHMARK formats=7 cold_generated=7 warm_reused=7 sources_unchanged=7"
    return $lines -join "`n"
}

$mediaName = (@(Get-AmeSyntheticCases | Where-Object Name -CEQ "media"))[0].Test
$mediaMarker = Get-AmeSyntheticMediaProtocolFixture
$mediaOutput = "running 1 test`ntest $mediaName ... $mediaMarker`nok`n`ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1160 filtered out; finished in 0.12s`n"
foreach ($valid in @($mediaOutput, $mediaOutput.Replace("`n", "`r`n"))) {
    Assert-AmeSyntheticResult -Output $valid -TestName $mediaName
}
$firstLine = ($mediaMarker -split "`n")[0]
$invalidEvidence = @(
    $mediaMarker.Replace("$firstLine`n", ""),
    $mediaMarker.Replace($firstLine, "$firstLine`n$firstLine"),
    $mediaMarker.Replace("format=png", "format=jpeg"),
    $mediaMarker.Replace("format=png", "format=avif"),
    $mediaMarker.Replace("format=ico source_width=256", "format=ico source_width=3000"),
    $mediaMarker.Replace("source_width=3000", "source_width=32"),
    $mediaMarker.Replace("preview_height=341", "preview_height=342"),
    $mediaMarker.Replace("bucket=512", "bucket=1024"),
    $mediaMarker.Replace("source_bytes=1024", "source_bytes=0"),
    $mediaMarker.Replace("source_bytes=1024", "source_bytes=33554433"),
    $mediaMarker.Replace("preview_bytes=2048", "preview_bytes=0"),
    $mediaMarker.Replace("preview_bytes=2048", "preview_bytes=8388609"),
    $mediaMarker.Replace("cache_bytes=2048", "cache_bytes=4096"),
    $mediaMarker.Replace("fixture_ms=23", "fixture_ms=360001"),
    $mediaMarker.Replace("cold_us=12000", "cold_us=360000001"),
    $mediaMarker.Replace("warm_us=400", "warm_us=360000001"),
    $mediaMarker.Replace("cold_us=12000", "cold_us=18446744073709551616"),
    $mediaMarker.Replace("cold_us=12000", "cold_us=-1"),
    $mediaMarker.Replace("cold_us=12000", "cold_us=1.5"),
    $mediaMarker.Replace("cold_us=12000", "cold_us=NaN"),
    $mediaMarker.Replace("cold_us=12000", ("cold_us=" + [char]0x0661)),
    $mediaMarker.Replace("cold_us=12000", "cold_us=12000 cold_us=12000"),
    $mediaMarker.Replace("source_unchanged=true", "source_unchanged=false"),
    $mediaMarker.Replace("cache_reused=true", "cache_reused=false"),
    $mediaMarker.Replace("warm_reused=7", "warm_reused=6"),
    $mediaMarker.Replace("sources_unchanged=7", "sources_unchanged=6"),
    $mediaMarker.Replace($firstLine, "$firstLine extra=1"),
    $mediaMarker.Replace("AME_MEDIA_BENCHMARK", "AME_MEDIA_FORMAT format=extra`nAME_MEDIA_BENCHMARK")
)
foreach ($invalid in $invalidEvidence) {
    $refused = $false
    try { Assert-AmeSyntheticResult -Output $mediaOutput.Replace($mediaMarker, $invalid) -TestName $mediaName }
    catch {
        if ($_.Exception.Message -notmatch '^Synthetic (media|execution)') { throw }
        $refused = $true
    }
    if (-not $refused) { throw "Synthetic media guard accepted malformed or incomplete evidence" }
}
Write-Host "AME_SYNTHETIC_MEDIA_PROTOCOL formats=7 negative_cases=$($invalidEvidence.Count) source_and_cache_proof=passed"
