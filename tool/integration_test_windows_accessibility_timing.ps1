[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_timing.ps1")

$timing = New-AmeWindowsUiaTiming
Set-AmeWindowsUiaTimingStage $timing "finding-elements" 20
Set-AmeWindowsUiaTimingStage $timing "reading-properties" 7200
Set-AmeWindowsUiaTimingStage $timing "finding-elements" 7240
Set-AmeWindowsUiaTimingStage $timing "complete" 7300
if ($timing.StageMilliseconds["finding-elements"] -ne 7240 -or
    $timing.StageMilliseconds["reading-properties"] -ne 40 -or
    $timing.StageMilliseconds["loading-assemblies"] -ne 20) {
    throw "Windows UIA timing lost repeated stages or misattributed the slow operation"
}
$record = [pscustomobject]@{
    stageMilliseconds = [pscustomobject]$timing.StageMilliseconds
    evidenceWriteMilliseconds = 50
}
Assert-AmeWindowsUiaTimingRecord $record
foreach ($malformed in @(
    '{"stageMilliseconds":{"unknown":1},"evidenceWriteMilliseconds":0}',
    '{"stageMilliseconds":{"finding-elements":-1},"evidenceWriteMilliseconds":0}',
    '{"stageMilliseconds":{"finding-elements":"1"},"evidenceWriteMilliseconds":0}',
    '{"stageMilliseconds":[],"evidenceWriteMilliseconds":0}',
    '{"stageMilliseconds":{},"evidenceWriteMilliseconds":-1}',
    '{"stageMilliseconds":{}}'
)) {
    $rejected = $false
    try { Assert-AmeWindowsUiaTimingRecord ($malformed | ConvertFrom-Json) } catch { $rejected = $true }
    if (-not $rejected) { throw "Windows UIA malformed timing was accepted" }
}
$rejected = $false
try { Set-AmeWindowsUiaTimingStage $timing "complete" 7299 } catch { $rejected = $true }
if (-not $rejected) { throw "Windows UIA timing accepted a backwards clock" }
Write-Host "Windows UIA bounded timing checks passed."
