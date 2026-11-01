$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-AmeClassificationVocabularyManifest {
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) "resources/classification/vocabulary-distribution.json"
    return [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8) | ConvertFrom-Json
}

function Get-AmeClassificationVocabularyPayloadNames {
    $manifest = Get-AmeClassificationVocabularyManifest
    foreach ($file in $manifest.files) {
        "$($manifest.bundleDirectory)/$($file.name)"
    }
}

function Invoke-AmeClassificationVocabularyPython {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $repository = Split-Path -Parent $PSScriptRoot
    $uv = Get-Command uv -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $uv) {
        $cache = Join-Path $repository "build/classification-vocabulary-python-cache"
        $python = & $uv.Source --cache-dir $cache python find ">=3.11" --offline
        if ($LASTEXITCODE -ne 0) { throw "Unable to resolve an installed vocabulary build interpreter" }
    } else {
        $python = (Get-Command python -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    }
    & $python -c "import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)"
    if ($LASTEXITCODE -ne 0) { throw "Classification vocabulary preparation requires installed Python 3.11 or later" }
    & $python -X utf8 @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Classification vocabulary verification or preparation failed" }
}

function Assert-AmeClassificationVocabularyDirectory {
    param([Parameter(Mandatory = $true)][string]$Path)

    Invoke-AmeClassificationVocabularyPython @(
        (Join-Path $PSScriptRoot "release_classification_vocabulary.py"), "--verify-bundle", $Path
    )
}
