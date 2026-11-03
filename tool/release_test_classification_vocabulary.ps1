$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_classification_vocabulary.ps1")

Invoke-AmeClassificationVocabularyPython @(
    (Join-Path $PSScriptRoot "release_test_classification_vocabulary.py")
)
