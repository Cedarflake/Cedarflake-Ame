$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_classification_models.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')

function Assert-ModelPackageRejected {
    param([scriptblock]$Action)
    $rejected = $false
    try { & $Action } catch { $rejected = $true }
    if (!$rejected) { throw 'Unsafe model package was admitted' }
}

$manifest = Get-AmeClassificationModelsManifest
Assert-AmeClassificationModelsManifest $manifest
if (@(Get-AmeClassificationModelsPayloadNames).Count -ne 5) { throw 'Installer model inventory is incomplete' }
foreach ($uri in @('http://huggingface.co/file', 'https://user@huggingface.co/file', 'https://huggingface.co:444/file', 'https://huggingface.co.invalid/file')) {
    Assert-ModelPackageRejected { Assert-AmeModelDownloadUri ([Uri]$uri) }
}
Assert-AmeModelDownloadUri ([Uri]'https://cas-bridge.xethub.hf.co/file')

Invoke-AmeAcceptanceScratch -Name 'classification-model-distribution' -Action {
    param($scratch)
    $source = Join-Path $scratch 'source'
    [void][IO.Directory]::CreateDirectory($source)
    $fixture = $manifest | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    foreach ($artifact in $fixture.files) {
        $path = Join-Path $source $artifact.name
        [IO.File]::WriteAllText($path, "fixture $($artifact.name)", [Text.UTF8Encoding]::new($false))
        $artifact.length = (Get-Item -LiteralPath $path).Length
        $artifact.sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $output = Join-Path $scratch 'prepared'
    New-AmeClassificationModelsDirectory $output $source $fixture
    Assert-AmeClassificationModelsDirectory $output $fixture
    New-AmeClassificationModelsDirectory $output $source $fixture
    foreach ($notice in $fixture.notices) {
        if (!(Test-Path -LiteralPath (Join-Path $output $notice.name))) { throw 'Model attribution was omitted' }
    }
    $tagged = Join-Path $scratch 'tagged-metadata'
    [void][IO.Directory]::CreateDirectory((Join-Path $tagged 'installation'))
    $taggedManifest = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    foreach ($notice in $taggedManifest.notices) {
        $path = Join-Path $tagged "installation/$($notice.name)"
        [IO.File]::WriteAllText($path, 'tagged model attribution', [Text.UTF8Encoding]::new($false))
        $notice.length = (Get-Item -LiteralPath $path).Length
        $notice.sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $taggedManifest | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $tagged 'model_distribution.json') -Encoding utf8
    $taggedOutput = Join-Path $scratch 'tagged-payload'
    & (Join-Path $PSScriptRoot 'release_prepare_classification_models.ps1') `
        -OutputDirectory $taggedOutput -SourceDirectory $source -MetadataDirectory $tagged
    Assert-AmeClassificationModelsDirectory $taggedOutput $taggedManifest
    if (@(Get-AmeClassificationModelsPayload $taggedOutput -Manifest $taggedManifest).Count -ne 5) {
        throw 'Tagged model metadata did not produce the complete installer inventory'
    }
    $first = Join-Path $output $fixture.files[0].name
    [IO.File]::WriteAllText($first, 'damaged', [Text.UTF8Encoding]::new($false))
    Assert-ModelPackageRejected { New-AmeClassificationModelsDirectory $output $source $fixture }
    [IO.File]::Copy((Join-Path $source $fixture.files[0].name), $first, $true)
    $extra = Join-Path $output 'unexpected.txt'
    [IO.File]::WriteAllText($extra, 'unexpected', [Text.UTF8Encoding]::new($false))
    Assert-ModelPackageRejected { Assert-AmeClassificationModelsDirectory $output $fixture }
    Remove-Item -LiteralPath $extra
    $missing = Join-Path $output $fixture.notices[1].name
    Remove-Item -LiteralPath $missing
    Assert-ModelPackageRejected { Assert-AmeClassificationModelsDirectory $output $fixture }
    $invalid = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $invalid.files[0].name = '../outside'
    Assert-ModelPackageRejected { New-AmeClassificationModelsDirectory (Join-Path $scratch 'unsafe') $source $invalid }
    Assert-ModelPackageRejected { New-AmeClassificationModelsDirectory (Split-Path -Parent $PSScriptRoot) $source $fixture }
    $interrupted = Join-Path $scratch 'interrupted'
    $brokenSource = Join-Path $source $fixture.files[1].name
    [IO.File]::WriteAllText($brokenSource, 'truncated', [Text.UTF8Encoding]::new($false))
    Assert-ModelPackageRejected { New-AmeClassificationModelsDirectory $interrupted $source $fixture }
    if ((Test-Path -LiteralPath $interrupted) -or @(Get-ChildItem -LiteralPath $scratch -Force | Where-Object Name -Like '.classification-models-*').Count -ne 0) {
        throw 'Failed model preparation left staging payloads behind'
    }
    $stream = [IO.MemoryStream]::new([byte[]](1, 2, 3, 4))
    try {
        Assert-ModelPackageRejected { Copy-AmeModelArtifactStream $stream (Join-Path $scratch 'oversized') 3 ([Threading.CancellationToken]::None) }
    } finally { $stream.Dispose() }
}
Write-Host 'Model payload reuse, notices, tamper, omission, bounds and interrupted cleanup passed'
